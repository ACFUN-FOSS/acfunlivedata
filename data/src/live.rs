use crate::{config::LiveConfig, sqlite::save_data};
use acfunliveapi::{
    client::{DefaultApiClient, DefaultApiClientBuilder},
    response::{
        Gift as ApiGift, LiveData as ApiLiveData, Summary as ApiSummary, UserInfo, UserLiveInfo,
    },
};
use acfunlivedanmaku::{
    acproto::{CommonStateSignalCurrentRedpackList, CommonStateSignalDisplayInfo},
    client::DanmakuClient,
    danmaku::*,
};
use acfunlivedata_common::{data::*, message::DataCenterMessage};
use ahash::{AHashMap, AHashSet};
use anyhow::{Context, Result};
use cached::proc_macro::cached;
use futures::StreamExt;
use once_cell::sync::{Lazy, OnceCell};
use std::{future::Future, sync::Arc, time::Duration};
use tokio::{sync::mpsc, time};

pub static LIVE_TX: OnceCell<mpsc::UnboundedSender<LiveMessage>> = OnceCell::new();
pub static ALL_LIVES_TX: OnceCell<mpsc::UnboundedSender<AllLiveData>> = OnceCell::new();
pub static GIFT_TX: OnceCell<mpsc::UnboundedSender<Vec<ApiGift>>> = OnceCell::new();

const LIVE_LIST_INTERVAL: Duration = Duration::from_secs(10);
const RETRY_INTERVAL: Duration = Duration::from_secs(2);
const TIMEOUT: Duration = Duration::from_secs(10);
const SUMMARY_WAIT: Duration = Duration::from_secs(10);
const SUMMARY_INTERVAL: Duration = Duration::from_secs(1800);

static API_CLIENT_BUILDER: Lazy<DefaultApiClientBuilder> = Lazy::new(|| {
    DefaultApiClientBuilder::default_client().expect("failed to construct ApiClientBuilder")
});

type FansCount = Option<i32>;
type MedalName = Option<String>;
type MedalCount = Option<i32>;

#[derive(Clone, Debug)]
pub enum LiveData {
    LiveInfo(LiveInfo),
    Title(Title),
    LiverInfo(LiverInfo),
    UpdateCount(LiveId, FansCount, MedalName, MedalCount),
    Summary(Summary),
    Comment(Comment),
    Follow(Follow),
    Gift(Gift),
    JoinClub(JoinClub),
    Banana(Option<String>),
    WatchingCount(CommonStateSignalDisplayInfo),
    Redpack(CommonStateSignalCurrentRedpackList),
    ChatCall(ChatCall),
    ChatReady(ChatReady),
    ChatEnd(ChatEnd),
    AuthorChatCall(AuthorChatCall),
    AuthorChatReady(AuthorChatReady),
    AuthorChatEnd(AuthorChatEnd),
    AuthorChatChangeSoundConfig(AuthorChatChangeSoundConfig),
    Stop,
}

#[derive(Clone, Debug)]
pub enum AllLiveData {
    Live(Live),
    Summary(LiveId, ApiSummary),
}

#[derive(Clone, Debug)]
pub enum LiveMessage {
    LiveList(Vec<UserLiveInfo>),
    StopDanmaku(LiveId),
    StopSummary(LiveId),
    Command(DataCenterMessage),
}

#[derive(Clone, Debug)]
struct LiveMapData {
    title: Option<String>,
    data_tx: mpsc::UnboundedSender<LiveData>,
}

#[cached(size = 1, time = 3600, result = true)]
async fn build_client() -> Result<DefaultApiClient> {
    Ok(API_CLIENT_BUILDER.clone().build().await?)
}

#[inline]
async fn run_thrice<F, Fut>(name: &str, f: F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<()>>,
{
    for i in 0..3 {
        if let Err(e) = f().await {
            log::warn!("{} error: {}: {}", name, e, e.root_cause());
        } else {
            return;
        }
        if i == 2 {
            log::error!("failed to run {} thrice", name);
        } else {
            time::sleep(RETRY_INTERVAL).await;
        }
    }
}

#[derive(Clone, Debug)]
struct Liver {
    live_id: LiveId,
    liver_uid: i64,
    data_tx: Option<mpsc::UnboundedSender<LiveData>>,
}

impl Liver {
    #[inline]
    fn new(live_id: String, liver_uid: i64) -> Self {
        Self {
            live_id: Arc::new(live_id),
            liver_uid,
            data_tx: None,
        }
    }

    #[inline]
    fn send_message<T>(&self, tx: &mpsc::UnboundedSender<T>, message: T) {
        if let Err(e) = tx.send(message) {
            log::error!("{} failed to send message through channel: {}", self, e);
        }
    }

    #[inline]
    fn send_data_message(&self, data: LiveData) {
        if let Some(tx) = &self.data_tx {
            self.send_message(tx, data);
        } else {
            unreachable!("{} data_tx is None", self);
        }
    }

    async fn gift(&self) -> Result<()> {
        let api_client = build_client()
            .await
            .with_context(|| format!("{} failed to build AcFun API client", self))?;
        let list = api_client
            .get_gift_list(&*self.live_id)
            .await
            .with_context(|| format!("{} failed to get gift list", self))?;
        let gift_tx = GIFT_TX.get().expect("failed to get GIFT_TX");
        self.send_message(gift_tx, list.data.gift_list);

        Ok(())
    }

    async fn danmaku(&self, live_data: ApiLiveData, liver_info: UserInfo) {
        let live_tx = LIVE_TX.get().expect("failed to get LIVE_TX");
        let api_client = match build_client().await {
            Ok(client) => client,
            Err(e) => {
                log::error!("{} failed to build AcFun api client: {}", self, e);
                self.send_message(live_tx, LiveMessage::StopDanmaku(self.live_id.clone()));
                return;
            }
        };
        // 获取直播信息
        let liver = self.clone();
        let api_client_ = api_client.clone();
        let _ = tokio::spawn(async move {
            run_thrice("live_info()", || {
                liver.live_info(&api_client_, live_data.clone(), liver_info.clone())
            })
            .await;
        });
        match DanmakuClient::from_api_client(&api_client, self.liver_uid).await {
            Ok(mut client) => {
                // live_id不相同的情况
                if client.live_id() != self.live_id.as_str() {
                    log::warn!(
                        "[{}] different live id, live data: {}, danmaku client: {}",
                        self.liver_uid,
                        self.live_id,
                        client.live_id()
                    );
                    self.send_message(live_tx, LiveMessage::StopDanmaku(self.live_id.clone()));
                    return;
                }
                log::info!("{} start getting danmaku", self);
                // 获取弹幕
                loop {
                    match time::timeout(TIMEOUT, client.next()).await {
                        Ok(option) => match option {
                            Some(result) => match result {
                                Ok(msg) => match msg {
                                    Danmaku::ActionSignal(signals) => self.action(signals),
                                    Danmaku::StateSignal(signals) => self.state(signals),
                                    Danmaku::NotifySignal(_) => {}
                                },
                                Err(e) => {
                                    log::warn!("{} getting danmaku error: {}", self, e);
                                    break;
                                }
                            },
                            None => break,
                        },
                        Err(_) => {
                            log::warn!("{} danmaku client timeout", self);
                            break;
                        }
                    }
                }
                if let Err(e) = client.close().await {
                    log::error!("{} failed to close WebSocket connection: {}", self, e);
                }
                log::info!("{} stop getting danmaku", self);
            }
            Err(e) => {
                log::warn!("{} failed to build AcFun danmaku client: {}", self, e);
            }
        }
        // 获取直播总结
        let liver = self.clone();
        let _ = tokio::spawn(async move {
            let api_client = api_client;
            run_thrice("summary()", || liver.summary(&api_client)).await;
        });
        self.send_message(live_tx, LiveMessage::StopDanmaku(self.live_id.clone()));
    }

    async fn live_info(
        &self,
        api_client: &DefaultApiClient,
        live_data: ApiLiveData,
        liver_info: UserInfo,
    ) -> Result<()> {
        let mut medal_name = None;
        let mut medal_count = None;
        // 获取主播的守护徽章信息
        if live_data.has_fans_club {
            let list = api_client
                .get_medal_rank_list(self.liver_uid)
                .await
                .with_context(|| format!("{} failed to get medal rank list", self))?;
            medal_name = Some(list.club_name);
            medal_count = Some(list.fans_total_count);
        }
        let title = live_data.title.clone();
        let live_info = LiveInfo::new(self.liver_uid, live_data);
        let liver_info = LiverInfo::new(&live_info, liver_info, medal_name, medal_count);
        let title = Title {
            live_id: self.live_id.clone(),
            save_time: liver_info.save_time,
            title,
        };
        self.send_data_message(LiveData::LiveInfo(live_info));
        self.send_data_message(LiveData::LiverInfo(liver_info));
        self.send_data_message(LiveData::Title(title));

        Ok(())
    }

    async fn summary(&self, api_client: &DefaultApiClient) -> Result<()> {
        // 获取直播总结
        let summary = api_client
            .get_summary(&*self.live_id)
            .await
            .with_context(|| format!("{} failed to get summary", self))?;
        self.send_data_message(LiveData::Summary(Summary::new(
            self.live_id.clone(),
            summary,
        )));
        time::sleep(SUMMARY_WAIT).await;
        // 获取主播的直播信息
        let info = api_client
            .get_user_live_info(self.liver_uid)
            .await
            .with_context(|| format!("{} failed to get user live info", self))?;
        let fans_count = Some(info.user.fan_count_value);
        if let Some(data) = &info.live_data {
            // 检查是否意外结束获取弹幕
            if data.live_id == self.live_id.as_str() {
                let live_tx = LIVE_TX.get().expect("failed to get LIVE_TX");
                self.send_message(live_tx, LiveMessage::LiveList(vec![info]));
            }
        }
        let mut medal_name = None;
        let mut medal_count = None;
        // 获取主播的守护徽章信息
        let list = api_client
            .get_medal_rank_list(self.liver_uid)
            .await
            .with_context(|| format!("{} failed to get medal rank list", self))?;
        if list.has_fans_club {
            // 主播可能在直播中拥有或改变守护徽章
            medal_name = Some(list.club_name);
            medal_count = Some(list.fans_total_count);
        }
        self.send_data_message(LiveData::UpdateCount(
            self.live_id.clone(),
            fans_count,
            medal_name,
            medal_count,
        ));
        // 发送停止sql运行的消息
        self.send_data_message(LiveData::Stop);

        Ok(())
    }

    fn action(&self, signals: Vec<ActionSignal>) {
        for signal in signals {
            match signal {
                ActionSignal::Comment(comment) => {
                    self.send_data_message(LiveData::Comment(Comment::new(
                        self.live_id.clone(),
                        comment,
                    )));
                }
                ActionSignal::FollowAuthor(follow) => {
                    self.send_data_message(LiveData::Follow(Follow::new(
                        self.live_id.clone(),
                        follow,
                    )));
                }
                ActionSignal::Gift(gift) => {
                    self.send_data_message(LiveData::Gift(Gift::new(self.live_id.clone(), gift)));
                }
                ActionSignal::JoinClub(join_club) => {
                    self.send_data_message(LiveData::JoinClub(JoinClub::new(
                        self.live_id.clone(),
                        join_club,
                    )));
                }
                _ => {}
            }
        }
    }

    fn state(&self, signals: Vec<StateSignal>) {
        for signal in signals {
            match signal {
                StateSignal::AcFunDisplayInfo(info) => {
                    self.send_data_message(LiveData::Banana(Some(info.banana_count)));
                }
                StateSignal::DisplayInfo(info) => {
                    self.send_data_message(LiveData::WatchingCount(info));
                }
                StateSignal::RedpackList(list) => {
                    self.send_data_message(LiveData::Redpack(list));
                }
                StateSignal::ChatCall(call) => {
                    self.send_data_message(LiveData::ChatCall(call.into()));
                }
                StateSignal::ChatReady(ready) => {
                    self.send_data_message(LiveData::ChatReady(ChatReady::new(
                        self.live_id.clone(),
                        ready,
                    )));
                }
                StateSignal::ChatEnd(end) => {
                    self.send_data_message(LiveData::ChatEnd(ChatEnd::new(
                        self.live_id.clone(),
                        end,
                    )));
                }
                StateSignal::AuthorChatCall(call) => {
                    self.send_data_message(LiveData::AuthorChatCall(AuthorChatCall::new(
                        self.live_id.clone(),
                        call,
                    )));
                }
                StateSignal::AuthorChatReady(ready) => {
                    self.send_data_message(LiveData::AuthorChatReady(AuthorChatReady::new(
                        self.live_id.clone(),
                        ready,
                    )));
                }
                StateSignal::AuthorChatEnd(end) => {
                    self.send_data_message(LiveData::AuthorChatEnd(AuthorChatEnd::new(
                        self.live_id.clone(),
                        end,
                    )));
                }
                StateSignal::AuthorChatChangeSoundConfig(config) => {
                    self.send_data_message(LiveData::AuthorChatChangeSoundConfig(
                        AuthorChatChangeSoundConfig::new(self.live_id.clone(), config),
                    ));
                }
                _ => {}
            }
        }
    }
}

impl std::fmt::Display for Liver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] [{}]", self.live_id, self.liver_uid)
    }
}

pub async fn all_lives() {
    let api_client = build_client().await.expect("failed to build api client");
    let live_tx = LIVE_TX.get().expect("failed to get LIVE_TX");
    let mut interval = time::interval(LIVE_LIST_INTERVAL);

    loop {
        // 定时获取直播间列表
        let _ = interval.tick().await;
        match api_client.get_live_list(1_000_000, 0).await {
            Ok(list) => {
                if let Err(e) = live_tx.send(LiveMessage::LiveList(list.live_list)) {
                    unreachable!("failed to send live list: {}", e);
                }
            }
            Err(e) => {
                log::warn!("failed to get the live list: {}", e);
            }
        }
    }
}

pub async fn all_danmaku(
    mut live_rx: mpsc::UnboundedReceiver<LiveMessage>,
    mut config: LiveConfig,
) {
    let all_lives_tx = ALL_LIVES_TX.get().expect("failed to get ALL_LIVE_TX");
    // 保存所有直播
    let mut all_lives: AHashSet<LiveId> = AHashSet::new();
    // 保存所有正在获取直播总结的live_id
    let mut all_summaries: AHashSet<LiveId> = AHashSet::new();
    // 保存要记录数据的直播
    let mut lives: AHashMap<LiveId, LiveMapData> = AHashMap::new();
    while let Some(msg) = live_rx.recv().await {
        match msg {
            LiveMessage::LiveList(list) => {
                // 不处理直播间列表为空的情况
                if list.is_empty() {
                    log::warn!("the live list is empty");
                    continue;
                }
                let mut new_all_lives: AHashSet<LiveId> = AHashSet::new();
                for info in list {
                    let liver_uid = info.author_id;
                    if let Some(live_data) = info.live_data {
                        let liver = Liver::new(live_data.live_id.clone(), liver_uid);
                        // 查看是否刚开播的直播
                        if !all_lives.contains(&liver.live_id) {
                            liver.send_message(
                                all_lives_tx,
                                AllLiveData::Live(Live::new(
                                    liver.liver_uid,
                                    &live_data,
                                    &info.user,
                                )),
                            );
                            let liver_ = liver.clone();
                            // 获取直播间礼物列表
                            let _ = tokio::spawn(async move {
                                run_thrice("gift()", || liver_.gift()).await;
                            });
                        }
                        let _ = new_all_lives.insert(liver.live_id.clone());
                        // 查看是否要记录数据的直播
                        if !config.contains(liver.liver_uid) {
                            continue;
                        }
                        // 要记录数据的情况
                        match lives.get(&liver.live_id) {
                            Some(old_data) => {
                                // 直播改标题的情况
                                if live_data.title != old_data.title {
                                    let data_tx = old_data.data_tx.clone();
                                    liver.send_message(
                                        &data_tx,
                                        LiveData::Title(Title::new(
                                            liver.live_id.clone(),
                                            live_data.title.clone(),
                                        )),
                                    );
                                    let _ = lives.insert(
                                        liver.live_id,
                                        LiveMapData {
                                            title: live_data.title,
                                            data_tx,
                                        },
                                    );
                                }
                            }
                            None => {
                                let title = live_data.title.clone();
                                let (data_tx, data_rx) = mpsc::unbounded_channel();
                                let mut liver_ = liver.clone();
                                liver_.data_tx = Some(data_tx.clone());
                                let user_info = info.user;
                                let _ = tokio::spawn(async move {
                                    liver_.danmaku(live_data, user_info).await
                                });
                                let live_id_ = liver.live_id.clone();
                                let _ = tokio::task::spawn_blocking(move || {
                                    save_data(data_rx, live_id_, liver_uid)
                                });
                                let _ = lives.insert(liver.live_id, LiveMapData { title, data_tx });
                            }
                        }
                    } else {
                        log::warn!("[{}] there is no live data in live info", liver_uid);
                    }
                }
                for live_id in all_lives {
                    // 直播结束获取直播总结
                    if !new_all_lives.contains(&live_id) && !all_summaries.contains(&live_id) {
                        let _ = all_summaries.insert(live_id.clone());
                        let _ = tokio::spawn(async move {
                            run_thrice("all_summary()", || all_summary(live_id.clone())).await;
                            let live_tx = LIVE_TX.get().expect("failed to get LIVE_TX");
                            if let Err(e) = live_tx.send(LiveMessage::StopSummary(live_id.clone()))
                            {
                                log::error!(
                                    "[{}] failed to send LiveMessage::StopSummary: {}",
                                    live_id,
                                    e
                                );
                            }
                        });
                    }
                }
                all_lives = new_all_lives;
            }
            LiveMessage::StopDanmaku(live_id) => {
                if lives.remove(&live_id).is_none() {
                    log::warn!("live ID {} wasn't in lives", live_id);
                }
            }
            LiveMessage::StopSummary(live_id) => {
                if !all_summaries.remove(&live_id) {
                    log::warn!("live ID {} wasn't in all_summaries", live_id);
                }
            }
            LiveMessage::Command(DataCenterMessage::AddLiver(liver_uid, tool)) => {
                config.add_liver(liver_uid, tool).await;
                if let Err(e) = config.save_config().await {
                    log::error!("failed to save config: {}", e);
                    return;
                }
            }
            LiveMessage::Command(DataCenterMessage::DeleteLiver(liver_uid, tool)) => {
                config.delete_liver(liver_uid, tool).await;
                if let Err(e) = config.save_config().await {
                    log::error!("failed to save config: {}", e);
                    return;
                }
            }
        }
    }

    unreachable!("failed to receive live message");
}

async fn all_summary(live_id: Arc<String>) -> Result<()> {
    let api_client = build_client()
        .await
        .with_context(|| format!("[{}] failed to build AcFun API client", live_id))?;
    let all_lives_tx = ALL_LIVES_TX.get().expect("failed to get ALL_LIVE_TX");
    // 为了获取可能被隐藏的直播间的直播总结
    loop {
        let summary = api_client
            .get_summary(&*live_id)
            .await
            .with_context(|| format!("[{}] failed to get summary", live_id))?;
        time::sleep(SUMMARY_WAIT).await;
        let new_summary = api_client
            .get_summary(&*live_id)
            .await
            .with_context(|| format!("[{}] failed to get summary", live_id))?;
        if summary.data == new_summary.data {
            all_lives_tx.send(AllLiveData::Summary(live_id.clone(), new_summary))?;
            break;
        }
        all_lives_tx.send(AllLiveData::Summary(live_id.clone(), new_summary))?;
        log::info!(
            "[{}] this live is still on, failed to get the final summary, retrying...",
            live_id
        );
        time::sleep(SUMMARY_INTERVAL).await;
    }

    Ok(())
}
