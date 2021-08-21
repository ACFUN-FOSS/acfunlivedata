use crate::{
    interval::WATCH_INTERVAL_TX,
    live::{AllLiveData, LiveData},
    sql::*,
};
use acfunliveapi::response::Gift as ApiGift;
use acfunlivedata_common::{create_dir, data::*, database::*};
use ahash::AHashSet;
use anyhow::Result;
use once_cell::sync::Lazy;
use rusqlite::{named_params, Connection, OpenFlags, OptionalExtension};
use std::{path::Path, sync::Arc};
use tokio::sync::mpsc;

static OPEN_FLAGS: Lazy<OpenFlags> = Lazy::new(|| {
    OpenFlags::SQLITE_OPEN_READ_WRITE
        | OpenFlags::SQLITE_OPEN_CREATE
        | OpenFlags::SQLITE_OPEN_NO_MUTEX
});

#[inline]
pub async fn create_db_dir() -> Result<()> {
    create_dir(&*DATABASE_DIRECTORY).await?;
    create_dir(&*LIVERS_DIRECTORY).await
}

#[inline]
fn connect<P: AsRef<Path>>(path: P) -> Result<Connection> {
    Ok(Connection::open_with_flags(path, *OPEN_FLAGS)?)
}

pub fn all_lives(mut all_lives_rx: mpsc::UnboundedReceiver<AllLiveData>) {
    let conn = connect(&*ACFUN_LIVE_DATABASE)
        .unwrap_or_else(|e| panic!("failed to connect {}: {}", ACFUN_LIVE_DATABASE_NAME, e));
    conn.execute_batch(CREATE_LIVE)
        .expect("failed to create live table");
    let mut live_stmt = conn
        .prepare(INSERT_LIVE)
        .expect("failed to prepare live statement");
    let mut summary_stmt = conn
        .prepare(UPDATE_LIVE)
        .expect("failed to prepare updating live statement");

    log::info!("start live sql");
    while let Some(data) = all_lives_rx.blocking_recv() {
        match data {
            AllLiveData::Live(live) => {
                if let Err(e) = live_stmt.execute(named_params! {
                    ":live_id": live.live_id,
                    ":liver_uid": live.liver_uid,
                    ":nickname": live.nickname,
                    ":stream_name": live.stream_name,
                    ":start_time": live.start_time,
                    ":title": live.title,
                    ":live_type_id": live.live_type.as_ref().map(|t| t.id),
                    ":live_type_name": live.live_type.as_ref().map(|t| &t.name),
                    ":live_type_category_id": live.live_type.as_ref().map(|t| t.category_id),
                    ":live_type_category_name": live.live_type.as_ref().map(|t| &t.category_name),
                    ":portrait": live.portrait,
                    ":panoramic": live.panoramic,
                    ":disable_danmaku_show": live.disable_danmaku_show,
                    ":duration": live.duration,
                    ":like_count": live.like_count,
                    ":watch_count": live.watch_count,
                }) {
                    log::error!(
                        "[{}] [{}] failed to insert live: {}",
                        live.live_id,
                        live.liver_uid,
                        e
                    );
                }
            }
            AllLiveData::Summary(live_id, summary) => {
                if let Err(e) = summary_stmt.execute(named_params! {
                    ":duration": summary.data.live_duration_ms,
                    ":like_count": summary.data.like_count,
                    ":watch_count": summary.data.watch_count,
                    ":live_id": live_id,
                }) {
                    log::error!("[{}] failed to update live: {}", live_id, e);
                }
            }
        }
    }
    unreachable!("failed to receive AllLiveData");
}

pub fn gift_info(mut gift_rx: mpsc::UnboundedReceiver<Vec<ApiGift>>) {
    let conn = connect(&*GIFT_DATABASE)
        .unwrap_or_else(|e| panic!("failed to connect {}: {}", GIFT_DATABASE_NAME, e));
    conn.execute_batch(CREATE_GIFT_INFO)
        .expect("failed to create gift_info table");
    let mut stmt = conn
        .prepare(INSERT_GIFT_INFO)
        .expect("failed to prepare gift_info statement");

    log::info!("start gift sql");
    while let Some(list) = gift_rx.blocking_recv() {
        for gift in list {
            let info: GiftInfo = gift.into();
            if let Err(e) = stmt.execute(named_params! {
                ":id": info.id,
                ":save_time": info.save_time,
                ":gift_id": info.gift_id,
                ":gift_name": info.gift_name,
                ":ar_live_name": info.ar_live_name,
                ":pay_wallet_type": info.pay_wallet_type,
                ":gift_price": info.gift_price,
                ":webp_pic": info.webp_pic,
                ":png_pic": info.png_pic,
                ":small_png_pic": info.small_png_pic,
                ":allow_batch_send_size_list": info.allow_batch_send_size_list,
                ":can_combo": info.can_combo,
                ":can_draw": info.can_draw,
                ":magic_face_id": info.magic_face_id,
                ":vup_ar_id": info.vup_ar_id,
                ":description": info.description,
                ":redpack_price": info.redpack_price,
                ":corner_marker_text": info.corner_marker_text,
            }) {
                log::error!("failed to insert gift_info: {}", e);
            }
        }
    }
    unreachable!("failed to receive Vec<Gift>");
}

pub fn save_data(mut data_rx: mpsc::UnboundedReceiver<LiveData>, live_id: LiveId, liver_uid: i64) {
    let path = liver_db_path(liver_uid);
    let conn = match Conn::new(&path, live_id.clone(), liver_uid) {
        Ok(conn) => conn,
        Err(e) => {
            log::error!(
                "[{}] [{}] failed to connect {}: {}",
                live_id,
                liver_uid,
                path.display(),
                e
            );
            return;
        }
    };
    if let Err(e) = conn.create_livers_table() {
        log::error!(
            "[{}] [{}] failed to create livers table: {}",
            live_id,
            liver_uid,
            e
        );
        return;
    }

    let mut interval_rx = WATCH_INTERVAL_TX.subscribe();
    let mut banana = None;
    let mut max_watch = None;
    let mut redpack_id: AHashSet<String> = AHashSet::new();
    log::info!("[{}] [{}] start saving data", live_id, liver_uid);
    while let Some(data) = data_rx.blocking_recv() {
        match data {
            LiveData::LiveInfo(info) => conn.live_info(info),
            LiveData::Title(title) => conn.title(title),
            LiveData::LiverInfo(info) => conn.liver_info(info),
            LiveData::UpdateCount(live_id, fans_count, medal_name, medal_count) => {
                conn.update_count(live_id, fans_count, medal_name, medal_count);
            }
            LiveData::Summary(summary) => conn.summary(summary, max_watch, banana.clone()),
            LiveData::Comment(comment) => conn.comment(comment),
            LiveData::Follow(follow) => conn.follow(follow),
            LiveData::Gift(gift) => conn.gift(gift),
            LiveData::JoinClub(join_club) => conn.join_club(join_club),
            LiveData::Banana(b) => banana = b,
            LiveData::WatchingCount(info) => {
                let count = if info.watching_count.contains('万') {
                    info.watching_count
                        .replace('万', "")
                        .parse::<f32>()
                        .map(|c| (c * 10_000.0) as i32)
                        .ok()
                } else {
                    info.watching_count.parse::<i32>().ok()
                };
                match (count, max_watch) {
                    (Some(c), None) => max_watch = Some(c),
                    (Some(c), Some(m)) => {
                        if c > m {
                            max_watch = Some(c);
                        }
                    }
                    _ => {}
                }
                if interval_rx.try_recv().is_ok() {
                    conn.watching_count(WatchingCount::new(live_id.clone(), count));
                }
            }
            LiveData::Redpack(list) => {
                for r in list.redpacks {
                    if !redpack_id.contains(&r.red_pack_id) {
                        redpack_id.insert(r.red_pack_id.clone());
                        conn.redpack(Redpack::new(live_id.clone(), r));
                    }
                }
            }
            LiveData::ChatCall(chat_call) => conn.chat_call(chat_call),
            LiveData::ChatReady(chat_ready) => conn.chat_ready(chat_ready),
            LiveData::ChatEnd(chat_end) => conn.chat_end(chat_end),
            LiveData::AuthorChatCall(chat_call) => conn.author_chat_call(chat_call),
            LiveData::AuthorChatReady(chat_ready) => conn.author_chat_ready(chat_ready),
            LiveData::AuthorChatEnd(chat_end) => conn.author_chat_end(chat_end),
            LiveData::AuthorChatChangeSoundConfig(config) => {
                conn.author_chat_change_sound_config(config)
            }
            LiveData::Stop => {
                log::info!("[{}] [{}] stop saving data", live_id, liver_uid);
                return;
            }
        }
    }
    log::warn!(
        "[{}] [{}] stop saving data accidentally",
        live_id,
        liver_uid
    );
}

macro_rules! cached_stmt {
    ($conn:expr, $sql:expr, $s:expr) => {
        match ($conn).conn.prepare_cached(($sql)) {
            Ok(stmt) => stmt,
            Err(e) => {
                log::error!("{} failed to prepare {} statement: {}", ($conn), ($s), e);
                return;
            }
        }
    };
}

#[derive(Debug)]
struct Conn {
    live_id: Arc<String>,
    liver_uid: i64,
    conn: Connection,
}

impl Conn {
    #[inline]
    fn new<P: AsRef<Path>>(path: P, live_id: Arc<String>, liver_uid: i64) -> Result<Self> {
        Ok(Self {
            live_id,
            liver_uid,
            conn: connect(&path)?,
        })
    }

    fn create_livers_table(&self) -> Result<()> {
        self.conn.execute_batch(CREATE_LIVE_INFO)?;
        self.conn.execute_batch(CREATE_TITLE)?;
        self.conn.execute_batch(CREATE_LIVER_INFO)?;
        self.conn.execute_batch(CREATE_SUMMARY)?;
        self.conn.execute_batch(CREATE_COMMENT)?;
        self.conn.execute_batch(CREATE_FOLLOW)?;
        self.conn.execute_batch(CREATE_GIFT)?;
        self.conn.execute_batch(CREATE_JOIN_CLUB)?;
        self.conn.execute_batch(CREATE_WATCHING_COUNT)?;
        self.conn.execute_batch(CREATE_REDPACK)?;
        self.conn.execute_batch(CREATE_CHAT_CALL)?;
        self.conn.execute_batch(CREATE_CHAT_READY)?;
        self.conn.execute_batch(CREATE_CHAT_END)?;
        self.conn.execute_batch(CREATE_AUTHOR_CHAT_CALL)?;
        self.conn.execute_batch(CREATE_AUTHOR_CHAT_READY)?;
        self.conn.execute_batch(CREATE_AUTHOR_CHAT_END)?;
        self.conn
            .execute_batch(CREATE_AUTHOR_CHAT_CHANGE_SOUND_CONFIG)?;

        Ok(())
    }

    fn live_info(&self, info: LiveInfo) {
        let mut stmt = cached_stmt!(self, INSERT_LIVE_INFO, "live_info");
        if let Err(e) = stmt.execute(named_params! {
            ":live_id": info.live_id,
            ":liver_uid": info.liver_uid,
            ":stream_name": info.stream_name,
            ":start_time": info.start_time,
            ":cover": info.cover,
            ":live_type_id": info.live_type.as_ref().map(|t| t.id),
            ":live_type_name": info.live_type.as_ref().map(|t| &t.name),
            ":live_type_category_id": info.live_type.as_ref().map(|t| t.category_id),
            ":live_type_category_name": info.live_type.as_ref().map(|t| &t.category_name),
            ":has_fans_club": info.has_fans_club,
            ":portrait": info.portrait,
            ":panoramic": info.panoramic,
            ":disable_danmaku_show": info.disable_danmaku_show,
            ":paid_show_user_buy_status": info.paid_show_user_buy_status,
        }) {
            log::error!("{} failed to insert live_info: {}", self, e);
        }
    }

    fn title(&self, title: Title) {
        let mut select_stmt = cached_stmt!(self, SELECT_TITLE, "selecting title");
        match select_stmt
            .query_row(named_params! {":live_id": title.live_id}, |r| {
                r.get::<_, Option<String>>(0)
            })
            .optional()
        {
            Ok(Some(t)) => {
                if t == title.title {
                    return;
                }
            }
            Ok(None) => {}
            Err(e) => {
                log::error!("{} failed to select title: {}", self, e);
                return;
            }
        }
        let mut insert_stmt = cached_stmt!(self, INSERT_TITLE, "inserting title");
        if let Err(e) = insert_stmt.execute(named_params! {
            ":live_id": title.live_id,
            ":save_time": title.save_time,
            ":title": title.title,
        }) {
            log::error!("{} failed to insert title: {}", self, e);
        }
    }

    fn liver_info(&self, info: LiverInfo) {
        let mut stmt = cached_stmt!(self, INSERT_LIVER_INFO, "liver_info");
        if let Err(e) = stmt.execute(named_params! {
            ":live_id": info.live_id,
            ":save_time": info.save_time,
            ":liver_uid": info.liver_uid,
            ":nickname": info.nickname,
            ":avatar": info.avatar,
            ":avatar_frame": info.avatar_frame,
            ":following_count": info.following_count,
            ":contribute_count": info.contribute_count,
            ":live_begin_fans_count": info.live_begin_fans_count,
            ":live_end_fans_count": info.live_end_fans_count,
            ":signature": info.signature,
            ":verified_text": info.verified_text,
            ":is_join_up_college": info.is_join_up_college,
            ":medal_name": info.medal_name,
            ":live_begin_medal_count": info.live_begin_medal_count,
            ":live_end_medal_count": info.live_end_medal_count,
        }) {
            log::error!("{} failed to insert liver_info: {}", self, e);
        }
    }

    fn update_count(
        &self,
        live_id: LiveId,
        fans_count: Option<i32>,
        medal_name: Option<String>,
        medal_count: Option<i32>,
    ) {
        let mut stmt = cached_stmt!(self, UPDATE_LIVER_INFO, "updating liver_info");
        if let Err(e) = stmt.execute(named_params! {
            ":fans_count": fans_count,
            ":medal_name": medal_name,
            ":medal_count": medal_count,
            ":live_id": live_id,
        }) {
            log::error!("{} failed to update liver_info: {}", self, e);
        }
    }

    fn summary(&self, summary: Summary, max_watch: Option<i32>, banana: Option<String>) {
        let mut stmt = cached_stmt!(self, REPLACE_SUMMARY, "summary");
        if let Err(e) = stmt.execute(named_params! {
            ":live_id": summary.live_id,
            ":save_time": summary.save_time,
            ":duration": summary.duration,
            ":like_count": summary.like_count,
            ":watch_total_count": summary.watch_total_count,
            ":watch_online_max_count": max_watch,
            ":banana_count": banana,
        }) {
            log::error!("{} failed to insert summary: {}", self, e);
        }
    }

    fn comment(&self, comment: Comment) {
        let mut stmt = cached_stmt!(self, INSERT_COMMENT, "comment");
        if let Err(e) = stmt.execute(named_params! {
            ":live_id": comment.live_id,
            ":send_time": comment.send_time,
            ":user_id": comment.user_info.as_ref().map(|u| u.user_id),
            ":nickname": comment.user_info.as_ref().map(|u| &u.nickname),
            ":avatar": comment.user_info.as_ref().map(|u| u.avatar.as_ref()).flatten(),
            ":medal_uper_uid": comment.user_info.as_ref().map(|u| u.medal.as_ref().map(|m| m.uper_uid)).flatten(),
            ":medal_name": comment.user_info.as_ref().map(|u| u.medal.as_ref().map(|m| &m.name)).flatten(),
            ":medal_level": comment.user_info.as_ref().map(|u| u.medal.as_ref().map(|m| m.level)).flatten(),
            ":manager": comment.user_info.as_ref().map(|u| u.manager).flatten(),
            ":content": comment.content,
        }) {
            log::error!("{} failed to insert comment: {}", self, e);
        }
    }

    fn follow(&self, follow: Follow) {
        let mut stmt = cached_stmt!(self, INSERT_FOLLOW, "follow");
        if let Err(e) = stmt.execute(named_params! {
            ":live_id": follow.live_id,
            ":send_time": follow.send_time,
            ":user_id": follow.user_info.as_ref().map(|u| u.user_id),
            ":nickname": follow.user_info.as_ref().map(|u| &u.nickname),
            ":avatar": follow.user_info.as_ref().map(|u| u.avatar.as_ref()).flatten(),
            ":medal_uper_uid": follow.user_info.as_ref().map(|u| u.medal.as_ref().map(|m| m.uper_uid)).flatten(),
            ":medal_name": follow.user_info.as_ref().map(|u| u.medal.as_ref().map(|m| &m.name)).flatten(),
            ":medal_level": follow.user_info.as_ref().map(|u| u.medal.as_ref().map(|m| m.level)).flatten(),
            ":manager": follow.user_info.as_ref().map(|u| u.manager).flatten(),
        }) {
            log::error!("{} failed to insert follow: {}", self, e);
        }
    }

    fn gift(&self, gift: Gift) {
        let mut stmt = cached_stmt!(self, INSERT_GIFT, "gift");
        if let Err(e) = stmt.execute(named_params! {
            ":live_id": gift.live_id,
            ":send_time": gift.send_time,
            ":user_id": gift.user_info.as_ref().map(|u| u.user_id),
            ":nickname": gift.user_info.as_ref().map(|u| &u.nickname),
            ":avatar": gift.user_info.as_ref().map(|u| u.avatar.as_ref()).flatten(),
            ":medal_uper_uid": gift.user_info.as_ref().map(|u| u.medal.as_ref().map(|m| m.uper_uid)).flatten(),
            ":medal_name": gift.user_info.as_ref().map(|u| u.medal.as_ref().map(|m| &m.name)).flatten(),
            ":medal_level": gift.user_info.as_ref().map(|u| u.medal.as_ref().map(|m| m.level)).flatten(),
            ":manager": gift.user_info.as_ref().map(|u| u.manager).flatten(),
            ":gift_id": gift.gift_id,
            ":count":gift.count,
            ":combo": gift.combo,
            ":value":gift.value,
            ":combo_id": gift.combo_id,
            ":slot_display_duration": gift.slot_display_duration,
            ":expire_duration": gift.expire_duration,
            ":draw_gift_info": gift.draw_gift_info,
        }) {
            log::error!("{} failed to insert gift: {}", self, e);
        }
    }

    fn join_club(&self, join_club: JoinClub) {
        let mut stmt = cached_stmt!(self, INSERT_JOIN_CLUB, "join_club");
        if let Err(e) = stmt.execute(named_params! {
            ":live_id": join_club.live_id,
            ":join_time": join_club.join_time,
            ":fans_uid": join_club.fans_info.as_ref().map(|u| u.user_id),
            ":fans_nickname": join_club.fans_info.as_ref().map(|u| &u.nickname),
            ":uper_uid": join_club.uper_info.as_ref().map(|u| u.user_id),
            ":uper_nickname": join_club.uper_info.as_ref().map(|u| &u.nickname),
        }) {
            log::error!("{} failed to insert join_club: {}", self, e);
        }
    }

    fn watching_count(&self, count: WatchingCount) {
        let mut stmt = cached_stmt!(self, INSERT_WATCHING_COUNT, "watching_count");
        if let Err(e) = stmt.execute(named_params! {
            ":live_id": count.live_id,
            ":save_time": count.save_time,
            ":watching_count": count.watching_count,
        }) {
            log::error!("{} failed to insert watching_count: {}", self, e);
        }
    }

    fn redpack(&self, redpack: Redpack) {
        let mut stmt = cached_stmt!(self, INSERT_REDPACK, "redpack");
        if let Err(e) = stmt.execute(named_params! {
            ":redpack_id": redpack.redpack_id,
            ":live_id": redpack.live_id,
            ":save_time": redpack.save_time,
            ":sender_user_id": redpack.sender_info.as_ref().map(|u| u.user_id),
            ":sender_nickname": redpack.sender_info.as_ref().map(|u| &u.nickname),
            ":sender_avatar": redpack.sender_info.as_ref().map(|u| u.avatar.as_ref()).flatten(),
            ":sender_medal_uper_uid": redpack.sender_info.as_ref().map(|u| u.medal.as_ref().map(|m| m.uper_uid)).flatten(),
            ":sender_medal_name": redpack.sender_info.as_ref().map(|u| u.medal.as_ref().map(|m| &m.name)).flatten(),
            ":sender_medal_level": redpack.sender_info.as_ref().map(|u| u.medal.as_ref().map(|m| m.level)).flatten(),
            ":sender_manager": redpack.sender_info.as_ref().map(|u| u.manager).flatten(),
            ":amount": redpack.amount,
            ":redpack_biz_unit": redpack.redpack_biz_unit,
            ":get_token_latest_time": redpack.get_token_latest_time,
            ":grab_begin_time": redpack.grab_begin_time,
            ":settle_begin_time": redpack.settle_begin_time,
        }) {
            log::error!("{} failed to insert redpack: {}", self, e);
        }
    }

    fn chat_call(&self, chat_call: ChatCall) {
        let mut stmt = cached_stmt!(self, INSERT_CHAT_CALL, "chat_call");
        if let Err(e) = stmt.execute(named_params! {
            ":chat_id": chat_call.chat_id,
            ":live_id": chat_call.live_id,
            ":call_time": chat_call.call_time,
        }) {
            log::error!("{} failed to insert chat_call: {}", self, e);
        }
    }

    fn chat_ready(&self, chat_ready: ChatReady) {
        let mut stmt = cached_stmt!(self, INSERT_CHAT_READY, "chat_ready");
        if let Err(e) = stmt.execute(named_params! {
            ":chat_id": chat_ready.chat_id,
            ":live_id": chat_ready.live_id,
            ":save_time": chat_ready.save_time,
            ":guest_user_id": chat_ready.guest_info.as_ref().map(|u| u.user_id),
            ":guest_nickname": chat_ready.guest_info.as_ref().map(|u| &u.nickname),
            ":guest_avatar": chat_ready.guest_info.as_ref().map(|u| u.avatar.as_ref()).flatten(),
            ":guest_medal_uper_uid": chat_ready.guest_info.as_ref().map(|u| u.medal.as_ref().map(|m| m.uper_uid)).flatten(),
            ":guest_medal_name": chat_ready.guest_info.as_ref().map(|u| u.medal.as_ref().map(|m| &m.name)).flatten(),
            ":guest_medal_level": chat_ready.guest_info.as_ref().map(|u| u.medal.as_ref().map(|m| m.level)).flatten(),
            ":guest_manager": chat_ready.guest_info.as_ref().map(|u| u.manager).flatten(),
            ":media_type": chat_ready.media_type,
        }) {
            log::error!("{} failed to insert chat_ready: {}", self, e);
        }
    }

    fn chat_end(&self, chat_end: ChatEnd) {
        let mut stmt = cached_stmt!(self, INSERT_CHAT_END, "chat_end");
        if let Err(e) = stmt.execute(named_params! {
            ":chat_id": chat_end.chat_id,
            ":live_id": chat_end.live_id,
            ":save_time": chat_end.save_time,
            ":end_type": chat_end.end_type,
        }) {
            log::error!("{} failed to insert chat_end: {}", self, e);
        }
    }

    fn author_chat_call(&self, chat_call: AuthorChatCall) {
        let mut stmt = cached_stmt!(self, INSERT_AUTHOR_CHAT_CALL, "author_chat_call");
        if let Err(e) = stmt.execute(named_params! {
            ":author_chat_id": chat_call.author_chat_id,
            ":live_id": chat_call.live_id,
            ":inviter_user_id": chat_call.inviter_info.as_ref().map(|i| i.player_info.as_ref()).flatten().map(|u| u.user_id),
            ":inviter_nickname": chat_call.inviter_info.as_ref().map(|i| i.player_info.as_ref()).flatten().map(|u| &u.nickname),
            ":inviter_avatar": chat_call.inviter_info.as_ref().map(|i| i.player_info.as_ref()).flatten().map(|u| u.avatar.as_ref()).flatten(),
            ":inviter_medal_uper_uid": chat_call.inviter_info.as_ref().map(|i| i.player_info.as_ref()).flatten().map(|u| u.medal.as_ref().map(|m| m.uper_uid)).flatten(),
            ":inviter_medal_name": chat_call.inviter_info.as_ref().map(|i| i.player_info.as_ref()).flatten().map(|u| u.medal.as_ref().map(|m| &m.name)).flatten(),
            ":inviter_medal_level": chat_call.inviter_info.as_ref().map(|i| i.player_info.as_ref()).flatten().map(|u| u.medal.as_ref().map(|m| m.level)).flatten(),
            ":inviter_manager": chat_call.inviter_info.as_ref().map(|i| i.player_info.as_ref()).flatten().map(|u| u.manager).flatten(),
            ":inviter_live_id": chat_call.inviter_info.as_ref().map(|i| &i.live_id),
            ":inviter_enable_jump_peer_live_room": chat_call.inviter_info.as_ref().map(|i| i.enable_jump_peer_live_room),
            ":call_time": chat_call.call_time,
        }) {
            log::error!("{} failed to insert author_chat_call: {}", self, e);
        }
    }

    fn author_chat_ready(&self, chat_ready: AuthorChatReady) {
        let mut stmt = cached_stmt!(self, INSERT_AUTHOR_CHAT_READY, "author_chat_ready");
        if let Err(e) = stmt.execute(named_params! {
            ":author_chat_id": chat_ready.author_chat_id,
            ":live_id": chat_ready.live_id,
            ":save_time": chat_ready.save_time,
            ":inviter_user_id": chat_ready.inviter_info.as_ref().map(|i| i.player_info.as_ref()).flatten().map(|u| u.user_id),
            ":inviter_nickname": chat_ready.inviter_info.as_ref().map(|i| i.player_info.as_ref()).flatten().map(|u| &u.nickname),
            ":inviter_avatar": chat_ready.inviter_info.as_ref().map(|i| i.player_info.as_ref()).flatten().map(|u| u.avatar.as_ref()).flatten(),
            ":inviter_medal_uper_uid": chat_ready.inviter_info.as_ref().map(|i| i.player_info.as_ref()).flatten().map(|u| u.medal.as_ref().map(|m| m.uper_uid)).flatten(),
            ":inviter_medal_name": chat_ready.inviter_info.as_ref().map(|i| i.player_info.as_ref()).flatten().map(|u| u.medal.as_ref().map(|m| &m.name)).flatten(),
            ":inviter_medal_level": chat_ready.inviter_info.as_ref().map(|i| i.player_info.as_ref()).flatten().map(|u| u.medal.as_ref().map(|m| m.level)).flatten(),
            ":inviter_manager": chat_ready.inviter_info.as_ref().map(|i| i.player_info.as_ref()).flatten().map(|u| u.manager).flatten(),
            ":inviter_live_id": chat_ready.inviter_info.as_ref().map(|i| &i.live_id),
            ":inviter_enable_jump_peer_live_room": chat_ready.inviter_info.as_ref().map(|i| i.enable_jump_peer_live_room),
            ":invitee_user_id": chat_ready.invitee_info.as_ref().map(|i| i.player_info.as_ref()).flatten().map(|u| u.user_id),
            ":invitee_nickname": chat_ready.invitee_info.as_ref().map(|i| i.player_info.as_ref()).flatten().map(|u| &u.nickname),
            ":invitee_avatar": chat_ready.invitee_info.as_ref().map(|i| i.player_info.as_ref()).flatten().map(|u| u.avatar.as_ref()).flatten(),
            ":invitee_medal_uper_uid": chat_ready.invitee_info.as_ref().map(|i| i.player_info.as_ref()).flatten().map(|u| u.medal.as_ref().map(|m| m.uper_uid)).flatten(),
            ":invitee_medal_name": chat_ready.invitee_info.as_ref().map(|i| i.player_info.as_ref()).flatten().map(|u| u.medal.as_ref().map(|m| &m.name)).flatten(),
            ":invitee_medal_level": chat_ready.invitee_info.as_ref().map(|i| i.player_info.as_ref()).flatten().map(|u| u.medal.as_ref().map(|m| m.level)).flatten(),
            ":invitee_manager": chat_ready.invitee_info.as_ref().map(|i| i.player_info.as_ref()).flatten().map(|u| u.manager).flatten(),
            ":invitee_live_id": chat_ready.invitee_info.as_ref().map(|i| &i.live_id),
            ":invitee_enable_jump_peer_live_room": chat_ready.invitee_info.as_ref().map(|i| i.enable_jump_peer_live_room),
        }) {
            log::error!("{} failed to insert author_chat_ready: {}", self, e);
        }
    }

    fn author_chat_end(&self, chat_end: AuthorChatEnd) {
        let mut stmt = cached_stmt!(self, INSERT_AUTHOR_CHAT_END, "author_chat_end");
        if let Err(e) = stmt.execute(named_params! {
            ":author_chat_id": chat_end.author_chat_id,
            ":live_id": chat_end.live_id,
            ":save_time": chat_end.save_time,
            ":end_type": chat_end.end_type,
            ":end_live_id": chat_end.end_live_id,
        }) {
            log::error!("{} failed to insert author_chat_end: {}", self, e);
        }
    }

    fn author_chat_change_sound_config(&self, config: AuthorChatChangeSoundConfig) {
        let mut stmt = cached_stmt!(
            self,
            INSERT_AUTHOR_CHAT_CHANGE_SOUND_CONFIG,
            "author_chat_change_sound_config"
        );
        if let Err(e) = stmt.execute(named_params! {
            ":author_chat_id": config.author_chat_id,
            ":live_id": config.live_id,
            ":save_time": config.save_time,
            ":sound_config_change_type": config.sound_config_change_type,
        }) {
            log::error!(
                "{} failed to insert author_chat_change_sound_config: {}",
                self,
                e
            );
        }
    }
}

impl std::fmt::Display for Conn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] [{}]", self.live_id, self.liver_uid)
    }
}
