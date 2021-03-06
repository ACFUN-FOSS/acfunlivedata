use crate::{
    config::{TokenInfo, User, CONFIG},
    pool::Connection,
    sql::*,
    sqlite::connect,
};
use acfunlivedata_common::{data::*, database::*};
use ahash::AHashMap;
use anyhow::{bail, Result};
use async_graphql::{
    validators::{InputValueValidator, IntGreaterThan},
    Context, Object, Value,
};
use cached::proc_macro::cached;
use rusqlite::ToSql;
use std::{iter, sync::Arc};

#[derive(Clone, Copy, Debug)]
pub struct QueryRoot;

macro_rules! compare_start_end {
    ($start:expr, $end:expr) => {
        if let (Some(start), Some(end)) = (($start), ($end)) {
            if start > end {
                bail!("start {} is bigger than end {}", start, end);
            }
        }
    };
}

macro_rules! sql_and_params {
    ($sql:expr; $(($varg:expr, $vsqlstr:expr)),*; $(($sarg:expr, $ssqlstr:expr)),*) => {{
        let mut sql = ($sql).to_string();
        let mut params: Vec<&dyn ToSql> = Vec::new();
        let mut where_or_and = iter::once(WHERE).chain(iter::repeat(AND));
        $(
            if let Some(varg) = &($varg) {
                let n = varg.len();
                for (i, arg) in varg.iter().enumerate() {
                    if i == 0 {
                        sql += where_or_and.next().unwrap();
                        sql += LEFT_PARENTHESES;
                    } else {
                        sql += OR;
                    }
                    sql += ($vsqlstr);
                    if i == n - 1 {
                        sql += RIGHT_PARENTHESES;
                    }
                    params.push(arg);
                }
            }
        )*
        $(
            if let Some(arg) = &($sarg) {
                sql += where_or_and.next().unwrap();
                sql += ($ssqlstr);
                params.push(arg);
            }
        )*
        sql += SEMICOLON;
        (sql, params)
    }}
}

macro_rules! get_pool {
    ($ctx:expr, $liver_uid:expr) => {{
        let liver_uid = {
            match (($ctx).data_unchecked::<User>(), &($liver_uid)) {
                (User::Admin, None) => bail!("admin token need liver_uid"),
                (User::Admin, Some(liver_uid)) => *liver_uid,
                (User::Liver(liver_uid), None) => *liver_uid,
                (User::Liver(_), Some(_)) => bail!("liver token don't need liver_uid"),
            }
        };
        connect(liver_db_path(liver_uid)).await?
    }};
}

#[derive(Clone, Copy, Debug)]
struct ListStringMinLength {
    length: i32,
}

impl InputValueValidator for ListStringMinLength {
    fn is_valid(&self, value: &Value) -> Result<(), String> {
        match value {
            Value::String(s) => {
                if s.len() < self.length as usize {
                    Err(format!(
                        "the string length is {}, must be greater than or equal to {}",
                        s.len(),
                        self.length
                    ))
                } else {
                    Ok(())
                }
            }
            Value::List(values) => {
                if values.iter().any(|v| {
                    if let Value::String(s) = v {
                        s.len() < self.length as usize
                    } else {
                        false
                    }
                }) {
                    Err(format!(
                        "the string length must be greater than or equal to {}",
                        self.length
                    ))
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ListIntGreaterThan {
    value: i64,
}

impl InputValueValidator for ListIntGreaterThan {
    fn is_valid(&self, value: &Value) -> Result<(), String> {
        match value {
            Value::Number(n) => {
                if let Some(n) = n.as_i64() {
                    if n <= self.value {
                        return Err(format!(
                            "the value is {}, must be greater than {}",
                            n, self.value
                        ));
                    }
                }
                Ok(())
            }
            Value::List(values) => {
                if values.iter().any(|v| {
                    if let Value::Number(n) = v {
                        if let Some(n) = n.as_i64() {
                            return n <= self.value;
                        }
                    }
                    false
                }) {
                    Err(format!("the value must be greater than {}", self.value))
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }
}

#[Object]
impl QueryRoot {
    #[graphql(visible = false)]
    async fn add_liver(
        &self,
        ctx: &Context<'_>,
        #[graphql(validator(IntGreaterThan(value = "0")))] liver_uid: i64,
    ) -> Result<TokenInfo> {
        if !ctx.data_unchecked::<User>().is_admin() {
            bail!("the admin authorization is needed");
        }
        let mut config = CONFIG.get().expect("failed to get CONFIG").write().await;
        let token = config.add_liver(liver_uid, false).await?;
        config.save_config().await?;

        Ok(token)
    }

    #[graphql(visible = false)]
    async fn delete_liver(
        &self,
        ctx: &Context<'_>,
        #[graphql(validator(IntGreaterThan(value = "0")))] liver_uid: i64,
    ) -> Result<TokenInfo> {
        if !ctx.data_unchecked::<User>().is_admin() {
            bail!("the admin authorization is needed");
        }
        let mut config = CONFIG.get().expect("failed to get CONFIG").write().await;
        let token = config.delete_liver(liver_uid, false).await?;
        config.save_config().await?;

        Ok(token)
    }

    #[inline]
    async fn liver_uid(&self, ctx: &Context<'_>) -> Result<i64> {
        match ctx.data_unchecked::<User>() {
            User::Admin => bail!("this is an admin token, liver uid doesn't exist"),
            User::Liver(liver_uid) => Ok(*liver_uid),
        }
    }

    #[graphql(visible = false)]
    async fn live(
        &self,
        ctx: &Context<'_>,
        #[graphql(validator(ListStringMinLength(length = "1")))] live_id: Option<Vec<String>>,
        #[graphql(validator(ListIntGreaterThan(value = "0")))] liver_uid: Option<Vec<i64>>,
        #[graphql(validator(IntGreaterThan(value = "-1")))] start: Option<i64>,
        #[graphql(validator(IntGreaterThan(value = "-1")))] end: Option<i64>,
    ) -> Result<Vec<Live>> {
        if !ctx.data_unchecked::<User>().is_admin() {
            bail!("the admin authorization is needed");
        }
        let pool = connect(ACFUN_LIVE_DATABASE.clone()).await?;

        tokio::task::spawn_blocking(move || {
            compare_start_end!(start, end);
            let (sql, params) = sql_and_params!(
                SELECT_LIVE;
                (live_id, LIVE_ID),
                (liver_uid, LIVER_UID);
                (start, START_TIME_START),
                (end, START_TIME_END)
            );

            let conn = futures::executor::block_on(pool.get())?;
            let mut stmt = conn.prepare_cached(&sql)?;
            let lives = stmt
                .query_map(params.as_slice(), |r| {
                    let live_type = match r.get::<_, Option<i32>>(6)? {
                        Some(id) => Some(LiveType {
                            id,
                            name: r.get(7)?,
                            category_id: r.get(8)?,
                            category_name: r.get(9)?,
                        }),
                        None => None,
                    };
                    Ok(Live {
                        live_id: Arc::new(r.get(0)?),
                        liver_uid: r.get(1)?,
                        nickname: r.get(2)?,
                        stream_name: r.get(3)?,
                        start_time: r.get(4)?,
                        title: r.get(5)?,
                        live_type,
                        portrait: r.get(10)?,
                        panoramic: r.get(11)?,
                        disable_danmaku_show: r.get(12)?,
                        duration: r.get(13)?,
                        like_count: r.get(14)?,
                        watch_count: r.get(15)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<Live>>>()?;

            Ok(lives)
        })
        .await?
    }

    #[inline]
    async fn gift_info(
        &self,
        #[graphql(validator(ListIntGreaterThan(value = "0")))] gift_id: Option<Vec<i64>>,
        #[graphql(visible = false)] all_history: Option<bool>,
    ) -> Result<Vec<GiftInfo>> {
        let gift_id = gift_id.map(|mut v| {
            v.sort_unstable();
            v.dedup();
            v
        });

        cache_gift_info(gift_id, all_history).await
    }

    async fn live_info(
        &self,
        ctx: &Context<'_>,
        #[graphql(validator(ListStringMinLength(length = "1")))] live_id: Option<Vec<String>>,
        #[graphql(validator(IntGreaterThan(value = "-1")))] start: Option<i64>,
        #[graphql(validator(IntGreaterThan(value = "-1")))] end: Option<i64>,
        #[graphql(validator(IntGreaterThan(value = "0")), visible = false)] liver_uid: Option<i64>,
    ) -> Result<Vec<LiveInfo>> {
        let pool = get_pool!(ctx, liver_uid);

        tokio::task::spawn_blocking(move || {
            compare_start_end!(start, end);
            let (sql, params) = sql_and_params!(
                SELECT_LIVE_INFO;
                (live_id, LIVE_ID);
                (start, START_TIME_START),
                (end, START_TIME_END)
            );

            let conn = futures::executor::block_on(pool.get())?;
            let mut stmt = conn.prepare_cached(&sql)?;
            let list = stmt
                .query_map(params.as_slice(), |r| {
                    let live_type = match r.get::<_, Option<i32>>(5)? {
                        Some(id) => Some(LiveType {
                            id,
                            name: r.get(6)?,
                            category_id: r.get(7)?,
                            category_name: r.get(8)?,
                        }),
                        None => None,
                    };
                    let live_id: Arc<String> = Arc::new(r.get(0)?);
                    let mut where_live_id = WHERE.to_string();
                    where_live_id.push_str(LIVE_ID);
                    where_live_id.push_str(SEMICOLON);
                    let params = &[(&live_id as &dyn ToSql)];

                    let mut sql = SELECT_TITLE.to_string();
                    sql.push_str(&where_live_id);
                    let titles = title(&conn, &sql, params)?;
                    sql.clear();
                    sql.push_str(SELECT_LIVER_INFO);
                    sql.push_str(&where_live_id);
                    let mut liver_info = liver_info(&conn, &sql, params)?;
                    sql.clear();
                    sql.push_str(SELECT_SUMMARY);
                    sql.push_str(&where_live_id);
                    let mut summaries = summary(&conn, &sql, params)?;

                    Ok(LiveInfo {
                        live_id,
                        liver_uid: r.get(1)?,
                        liver_info: liver_info.pop(),
                        stream_name: r.get(2)?,
                        start_time: r.get(3)?,
                        title: Some(titles),
                        cover: r.get(4)?,
                        live_type,
                        has_fans_club: r.get(9)?,
                        portrait: r.get(10)?,
                        panoramic: r.get(11)?,
                        disable_danmaku_show: r.get(12)?,
                        paid_show_user_buy_status: r.get(13)?,
                        summary: summaries.pop(),
                    })
                })?
                .collect::<rusqlite::Result<Vec<LiveInfo>>>()?;

            Ok(list)
        })
        .await?
    }

    async fn title(
        &self,
        ctx: &Context<'_>,
        #[graphql(validator(ListStringMinLength(length = "1")))] live_id: Option<Vec<String>>,
        #[graphql(validator(IntGreaterThan(value = "-1")))] start: Option<i64>,
        #[graphql(validator(IntGreaterThan(value = "-1")))] end: Option<i64>,
        #[graphql(validator(IntGreaterThan(value = "0")), visible = false)] liver_uid: Option<i64>,
    ) -> Result<Vec<Title>> {
        let pool = get_pool!(ctx, liver_uid);

        tokio::task::spawn_blocking(move || {
            compare_start_end!(start, end);
            let (sql, params) = sql_and_params!(
                SELECT_TITLE;
                (live_id, LIVE_ID);
                (start, SAVE_TIME_START),
                (end, SAVE_TIME_END)
            );

            let conn = futures::executor::block_on(pool.get())?;
            Ok(title(&conn, &sql, &params)?)
        })
        .await?
    }

    async fn liver_info(
        &self,
        ctx: &Context<'_>,
        #[graphql(validator(ListStringMinLength(length = "1")))] live_id: Option<Vec<String>>,
        #[graphql(validator(IntGreaterThan(value = "-1")))] start: Option<i64>,
        #[graphql(validator(IntGreaterThan(value = "-1")))] end: Option<i64>,
        #[graphql(validator(IntGreaterThan(value = "0")), visible = false)] liver_uid: Option<i64>,
    ) -> Result<Vec<LiverInfo>> {
        let pool = get_pool!(ctx, liver_uid);

        tokio::task::spawn_blocking(move || {
            compare_start_end!(start, end);
            let (sql, params) = sql_and_params!(
                SELECT_LIVER_INFO;
                (live_id, LIVE_ID);
                (start, SAVE_TIME_START),
                (end, SAVE_TIME_END)
            );

            let conn = futures::executor::block_on(pool.get())?;
            Ok(liver_info(&conn, &sql, &params)?)
        })
        .await?
    }

    async fn summary(
        &self,
        ctx: &Context<'_>,
        #[graphql(validator(ListStringMinLength(length = "1")))] live_id: Option<Vec<String>>,
        #[graphql(validator(IntGreaterThan(value = "-1")))] start: Option<i64>,
        #[graphql(validator(IntGreaterThan(value = "-1")))] end: Option<i64>,
        #[graphql(validator(IntGreaterThan(value = "0")), visible = false)] liver_uid: Option<i64>,
    ) -> Result<Vec<Summary>> {
        let pool = get_pool!(ctx, liver_uid);

        tokio::task::spawn_blocking(move || {
            compare_start_end!(start, end);
            let (sql, params) = sql_and_params!(
                SELECT_SUMMARY;
                (live_id, LIVE_ID);
                (start, SAVE_TIME_START),
                (end, SAVE_TIME_END)
            );

            let conn = futures::executor::block_on(pool.get())?;
            Ok(summary(&conn, &sql, &params)?)
        })
        .await?
    }

    #[allow(clippy::too_many_arguments)]
    async fn comment(
        &self,
        ctx: &Context<'_>,
        #[graphql(validator(ListStringMinLength(length = "1")))] live_id: Option<Vec<String>>,
        #[graphql(validator(ListIntGreaterThan(value = "0")))] user_id: Option<Vec<i64>>,
        #[graphql(validator(IntGreaterThan(value = "-1")))] start: Option<i64>,
        #[graphql(validator(IntGreaterThan(value = "-1")))] end: Option<i64>,
        #[graphql(validator(IntGreaterThan(value = "0")), visible = false)] liver_uid: Option<i64>,
    ) -> Result<Vec<Comment>> {
        let pool = get_pool!(ctx, liver_uid);

        tokio::task::spawn_blocking(move || {
            compare_start_end!(start, end);
            let (sql, params) = sql_and_params!(
                SELECT_COMMENT;
                (live_id, LIVE_ID),
                (user_id, USER_ID);
                (start, SEND_TIME_START),
                (end, SEND_TIME_END)
            );

            let conn = futures::executor::block_on(pool.get())?;
            let mut stmt = conn.prepare_cached(&sql)?;
            let comments = stmt
                .query_map(params.as_slice(), |r| {
                    let medal = match r.get::<_, Option<i64>>(5)? {
                        Some(uper_uid) => Some(MedalInfo {
                            uper_uid,
                            name: r.get(6)?,
                            level: r.get(7)?,
                        }),
                        None => None,
                    };
                    let user_info = match r.get::<_, Option<i64>>(2)? {
                        Some(user_id) => Some(UserInfo {
                            user_id,
                            nickname: r.get(3)?,
                            avatar: r.get(4)?,
                            medal,
                            manager: r.get(8)?,
                        }),
                        None => None,
                    };
                    Ok(Comment {
                        live_id: Arc::new(r.get(0)?),
                        send_time: r.get(1)?,
                        user_info,
                        content: r.get(9)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<Comment>>>()?;

            Ok(comments)
        })
        .await?
    }

    async fn follow(
        &self,
        ctx: &Context<'_>,
        #[graphql(validator(ListStringMinLength(length = "1")))] live_id: Option<Vec<String>>,
        #[graphql(validator(IntGreaterThan(value = "-1")))] start: Option<i64>,
        #[graphql(validator(IntGreaterThan(value = "-1")))] end: Option<i64>,
        #[graphql(validator(IntGreaterThan(value = "0")), visible = false)] liver_uid: Option<i64>,
    ) -> Result<Vec<Follow>> {
        let pool = get_pool!(ctx, liver_uid);

        tokio::task::spawn_blocking(move || {
            compare_start_end!(start, end);
            let (sql, params) = sql_and_params!(
                SELECT_FOLLOW;
                (live_id, LIVE_ID);
                (start, SEND_TIME_START),
                (end, SEND_TIME_END)
            );

            let conn = futures::executor::block_on(pool.get())?;
            let mut stmt = conn.prepare_cached(&sql)?;
            let list = stmt
                .query_map(params.as_slice(), |r| {
                    let medal = match r.get::<_, Option<i64>>(5)? {
                        Some(uper_uid) => Some(MedalInfo {
                            uper_uid,
                            name: r.get(6)?,
                            level: r.get(7)?,
                        }),
                        None => None,
                    };
                    let user_info = match r.get::<_, Option<i64>>(2)? {
                        Some(user_id) => Some(UserInfo {
                            user_id,
                            nickname: r.get(3)?,
                            avatar: r.get(4)?,
                            medal,
                            manager: r.get(8)?,
                        }),
                        None => None,
                    };
                    Ok(Follow {
                        live_id: Arc::new(r.get(0)?),
                        send_time: r.get(1)?,
                        user_info,
                    })
                })?
                .collect::<rusqlite::Result<Vec<Follow>>>()?;

            Ok(list)
        })
        .await?
    }

    #[allow(clippy::too_many_arguments)]
    async fn gift(
        &self,
        ctx: &Context<'_>,
        #[graphql(validator(ListStringMinLength(length = "1")))] live_id: Option<Vec<String>>,
        #[graphql(validator(ListIntGreaterThan(value = "0")))] user_id: Option<Vec<i64>>,
        #[graphql(validator(ListIntGreaterThan(value = "0")))] gift_id: Option<Vec<i64>>,
        #[graphql(validator(IntGreaterThan(value = "-1")))] start: Option<i64>,
        #[graphql(validator(IntGreaterThan(value = "-1")))] end: Option<i64>,
        #[graphql(validator(IntGreaterThan(value = "0")), visible = false)] liver_uid: Option<i64>,
    ) -> Result<Vec<Gift>> {
        let pool = get_pool!(ctx, liver_uid);

        tokio::task::spawn_blocking(move || {
            compare_start_end!(start, end);
            let (sql, params) = sql_and_params!(
                SELECT_GIFT;
                (live_id, LIVE_ID),
                (user_id, USER_ID),
                (gift_id, GIFT_ID);
                (start, SEND_TIME_START),
                (end, SEND_TIME_END)
            );

            let conn = futures::executor::block_on(pool.get())?;
            let mut stmt = conn.prepare_cached(&sql)?;
            let gifts = stmt
                .query_map(params.as_slice(), |r| {
                    let medal = match r.get::<_, Option<i64>>(5)? {
                        Some(uper_uid) => Some(MedalInfo {
                            uper_uid,
                            name: r.get(6)?,
                            level: r.get(7)?,
                        }),
                        None => None,
                    };
                    let user_info = match r.get::<_, Option<i64>>(2)? {
                        Some(user_id) => Some(UserInfo {
                            user_id,
                            nickname: r.get(3)?,
                            avatar: r.get(4)?,
                            medal,
                            manager: r.get(8)?,
                        }),
                        None => None,
                    };
                    Ok(Gift {
                        live_id: Arc::new(r.get(0)?),
                        send_time: r.get(1)?,
                        user_info,
                        gift_id: r.get(9)?,
                        count: r.get(10)?,
                        combo: r.get(11)?,
                        value: r.get(12)?,
                        combo_id: r.get(13)?,
                        slot_display_duration: r.get(14)?,
                        expire_duration: r.get(15)?,
                        draw_gift_info: r.get(16)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<Gift>>>()?;

            Ok(gifts)
        })
        .await?
    }

    async fn join_club(
        &self,
        ctx: &Context<'_>,
        #[graphql(validator(ListStringMinLength(length = "1")))] live_id: Option<Vec<String>>,
        #[graphql(validator(IntGreaterThan(value = "-1")))] start: Option<i64>,
        #[graphql(validator(IntGreaterThan(value = "-1")))] end: Option<i64>,
        #[graphql(validator(IntGreaterThan(value = "0")), visible = false)] liver_uid: Option<i64>,
    ) -> Result<Vec<JoinClub>> {
        let pool = get_pool!(ctx, liver_uid);

        tokio::task::spawn_blocking(move || {
            compare_start_end!(start, end);
            let (sql, params) = sql_and_params!(
                SELECT_JOIN_CLUB;
                (live_id, LIVE_ID);
                (start, JOIN_TIME_START),
                (end, JOIN_TIME_END)
            );

            let conn = futures::executor::block_on(pool.get())?;
            let mut stmt = conn.prepare_cached(&sql)?;
            let list = stmt
                .query_map(params.as_slice(), |r| {
                    let fans_info = match r.get::<_, Option<i64>>(2)? {
                        Some(user_id) => Some(AcFunUserInfo {
                            user_id,
                            nickname: r.get(3)?,
                        }),
                        None => None,
                    };
                    let uper_info = match r.get::<_, Option<i64>>(4)? {
                        Some(user_id) => Some(AcFunUserInfo {
                            user_id,
                            nickname: r.get(5)?,
                        }),
                        None => None,
                    };
                    Ok(JoinClub {
                        live_id: Arc::new(r.get(0)?),
                        join_time: r.get(1)?,
                        fans_info,
                        uper_info,
                    })
                })?
                .collect::<rusqlite::Result<Vec<JoinClub>>>()?;

            Ok(list)
        })
        .await?
    }

    async fn watching_count(
        &self,
        ctx: &Context<'_>,
        #[graphql(validator(ListStringMinLength(length = "1")))] live_id: Option<Vec<String>>,
        #[graphql(validator(IntGreaterThan(value = "-1")))] start: Option<i64>,
        #[graphql(validator(IntGreaterThan(value = "-1")))] end: Option<i64>,
        #[graphql(validator(IntGreaterThan(value = "0")), visible = false)] liver_uid: Option<i64>,
    ) -> Result<Vec<WatchingCount>> {
        let pool = get_pool!(ctx, liver_uid);

        tokio::task::spawn_blocking(move || {
            compare_start_end!(start, end);
            let (sql, params) = sql_and_params!(
                SELECT_WATCHING_COUNT;
                (live_id, LIVE_ID);
                (start, SAVE_TIME_START),
                (end, SAVE_TIME_END)
            );

            let conn = futures::executor::block_on(pool.get())?;
            let mut stmt = conn.prepare_cached(&sql)?;
            let list = stmt
                .query_map(params.as_slice(), |r| {
                    Ok(WatchingCount {
                        live_id: Arc::new(r.get(0)?),
                        save_time: r.get(1)?,
                        watching_count: r.get(2)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<WatchingCount>>>()?;

            Ok(list)
        })
        .await?
    }
}

#[cached(size = 20, time = 1800, result = true)]
async fn cache_gift_info(
    gift_id: Option<Vec<i64>>,
    all_history: Option<bool>,
) -> Result<Vec<GiftInfo>> {
    let pool = connect(GIFT_DATABASE.clone()).await?;

    tokio::task::spawn_blocking(move || {
        let (mut sql, params) = sql_and_params!(
            SELECT_GIFT_INFO;
            (gift_id, GIFT_ID);
        );
        let _ = sql.pop();
        sql += ORDER_SAVE_TIME_DESC;
        sql += SEMICOLON;

        let conn = futures::executor::block_on(pool.get())?;
        let mut list = gift_info(&conn, &sql, &params)?;

        if !all_history.unwrap_or(false) {
            list = list
                .into_iter()
                .fold(AHashMap::<i64, GiftInfo>::new(), |mut m, g| {
                    match m.get(&g.gift_id) {
                        Some(og) => {
                            if g.save_time > og.save_time {
                                let _ = m.insert(g.gift_id, g);
                            }
                        }
                        None => {
                            let _ = m.insert(g.gift_id, g);
                        }
                    }
                    m
                })
                .into_iter()
                .map(|(_, v)| v)
                .collect::<Vec<_>>();
            list.sort_unstable_by_key(|g| g.gift_id);
        } else {
            list.sort_by_key(|g| g.gift_id);
        }

        Ok(list)
    })
    .await?
}

#[inline]
fn gift_info(
    conn: &Connection,
    sql: &str,
    params: &[&dyn ToSql],
) -> rusqlite::Result<Vec<GiftInfo>> {
    let mut stmt = conn.prepare_cached(sql)?;
    let list = stmt
        .query_map(params, |r| {
            Ok(GiftInfo {
                id: r.get(0)?,
                save_time: r.get(1)?,
                gift_id: r.get(2)?,
                gift_name: r.get(3)?,
                ar_live_name: r.get(4)?,
                pay_wallet_type: r.get(5)?,
                gift_price: r.get(6)?,
                webp_pic: r.get(7)?,
                png_pic: r.get(8)?,
                small_png_pic: r.get(9)?,
                allow_batch_send_size_list: r.get(10)?,
                can_combo: r.get(11)?,
                can_draw: r.get(12)?,
                magic_face_id: r.get(13)?,
                vup_ar_id: r.get(14)?,
                description: r.get(15)?,
                redpack_price: r.get(16)?,
                corner_marker_text: r.get(17)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<GiftInfo>>>()?;

    Ok(list)
}

#[inline]
fn title(conn: &Connection, sql: &str, params: &[&dyn ToSql]) -> rusqlite::Result<Vec<Title>> {
    let mut stmt = conn.prepare_cached(sql)?;
    let titles = stmt
        .query_map(params, |r| {
            Ok(Title {
                live_id: Arc::new(r.get(0)?),
                save_time: r.get(1)?,
                title: r.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<Title>>>()?;

    Ok(titles)
}

#[inline]
fn liver_info(
    conn: &Connection,
    sql: &str,
    params: &[&dyn ToSql],
) -> rusqlite::Result<Vec<LiverInfo>> {
    let mut stmt = conn.prepare_cached(sql)?;
    let list = stmt
        .query_map(params, |r| {
            Ok(LiverInfo {
                live_id: Arc::new(r.get(0)?),
                save_time: r.get(1)?,
                liver_uid: r.get(2)?,
                nickname: r.get(3)?,
                avatar: r.get(4)?,
                avatar_frame: r.get(5)?,
                following_count: r.get(6)?,
                contribute_count: r.get(7)?,
                live_begin_fans_count: r.get(8)?,
                live_end_fans_count: r.get(9)?,
                signature: r.get(10)?,
                verified_text: r.get(11)?,
                is_join_up_college: r.get(12)?,
                medal_name: r.get(13)?,
                live_begin_medal_count: r.get(14)?,
                live_end_medal_count: r.get(15)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<LiverInfo>>>()?;

    Ok(list)
}

#[inline]
fn summary(conn: &Connection, sql: &str, params: &[&dyn ToSql]) -> rusqlite::Result<Vec<Summary>> {
    let mut stmt = conn.prepare_cached(sql)?;
    let summaries = stmt
        .query_map(params, |r| {
            Ok(Summary {
                live_id: Arc::new(r.get(0)?),
                save_time: r.get(1)?,
                duration: r.get(2)?,
                like_count: r.get(3)?,
                watch_total_count: r.get(4)?,
                watch_online_max_count: r.get(5)?,
                banana_count: r.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<Summary>>>()?;

    Ok(summaries)
}
