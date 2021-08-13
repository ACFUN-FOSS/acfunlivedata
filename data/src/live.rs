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
use futures::StreamExt;
use once_cell::sync::{Lazy, OnceCell};
use std::{future::Future, sync::Arc, time::Duration};
use tokio::{sync::mpsc, sync::Mutex, time};

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
static API_CLIENT_FACTORY: Lazy<ApiClientFactory> = Lazy::new(ApiClientFactory::new);

#[derive(Clone, Debug)]
pub enum LiveData {
    LiveInfo(LiveInfo),
    Title(Title),
    LiverInfo(LiverInfo),
    UpdateCount(LiveID, FansCount, MedalName, MedalCount),
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
    ViolationAlert(ViolationAlert),
    Stop,
}

#[derive(Clone, Debug)]
pub enum AllLiveData {
    Live(Live),
    Summary(LiveID, ApiSummary),
}

#[derive(Clone, Debug)]
pub enum LiveMessage {
    LiveList(Vec<UserLiveInfo>),
    StopDanmaku(LiveID),
    StopSummary(LiveID),
    Command(DataCenterMessage),
}

#[derive(Clone, Debug)]
struct LiveMapData {
    title: Option<String>,
    data_tx: mpsc::UnboundedSender<LiveData>,
}

#[derive(Clone, Debug)]
struct ApiClientFactory {
    num: Arc<Mutex<usize>>,
}

impl ApiClientFactory {
    const MAX: usize = 10;
    const WAIT: Duration = Duration::from_secs(1);

    #[inline]
    fn new() -> Self {
        Self {
            num: Arc::new(Mutex::new(0)),
        }
    }

    #[inline]
    async fn build(&self) -> Result<DefaultApiClient> {
        {
            // 匿名登陆短时间内不能太频繁，不然会出错
            let mut num = self.num.lock().await;
            *num += 1;
            if *num > Self::MAX {
                *num = 0;
                time::sleep(Self::WAIT).await;
            }
        }

        Ok(API_CLIENT_BUILDER.clone().build().await?)
    }
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

#[inline]
fn send_message<T>(tx: &mpsc::UnboundedSender<T>, live_id: &str, liver_uid: i64, message: T) {
    if let Err(e) = tx.send(message) {
        log::error!(
            "[{}] [{}] failed to send message through channel: {}",
            live_id,
            liver_uid,
            e
        );
    }
}

pub async fn all_lives() {
    let api_client = API_CLIENT_FACTORY
        .build()
        .await
        .expect("failed to build api client");
    let live_tx = LIVE_TX.get().expect("failed to get LIVE_TX");
    let mut interval = time::interval(LIVE_LIST_INTERVAL);

    loop {
        // 定时获取直播间列表
        let _ = interval.tick().await;
        match api_client.get_live_list(1_000_000, 0).await {
            Ok(list) => {
                if let Err(e) = live_tx.send(LiveMessage::LiveList(list.live_list)) {
                    log::error!("failed to send live list: {}", e);
                    return;
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
    let mut all_lives: AHashSet<String> = AHashSet::new();
    // 保存所有正在获取直播总结的live_id
    let mut all_summaries: AHashSet<String> = AHashSet::new();
    // 保存要记录数据的直播
    let mut lives: AHashMap<String, LiveMapData> = AHashMap::new();
    while let Some(msg) = live_rx.recv().await {
        match msg {
            LiveMessage::LiveList(list) => {
                //let a = list
                //    .iter()
                //    .map(|i| i.live_data.as_ref().map(|j| &j.live_id))
                //    .collect::<Vec<_>>();
                //println!("list {:?}", a);
                // 不处理直播间列表为空的情况
                if list.is_empty() {
                    log::warn!("the live list is empty");
                    continue;
                }
                let mut new_all_lives: AHashSet<String> = AHashSet::new();
                for info in list {
                    let liver_uid = info.author_id;
                    if let Some(live_data) = info.live_data {
                        let live_id = live_data.live_id.clone();
                        // 查看是否刚开播的直播
                        if !all_lives.contains(&live_id) {
                            send_message(
                                all_lives_tx,
                                &live_id,
                                liver_uid,
                                AllLiveData::Live(Live::new(liver_uid, &live_data, &info.user)),
                            );
                            let live_id_ = live_id.clone();
                            // 获取直播间礼物列表
                            let _ = tokio::spawn(async move {
                                let live_id = live_id_;
                                run_thrice("gift()", || gift(&live_id, liver_uid)).await;
                            });
                        }
                        let _ = new_all_lives.insert(live_id.clone());
                        // 查看是否要记录数据的直播
                        if !config.contains(liver_uid) {
                            continue;
                        }
                        // 要记录数据的情况
                        match lives.get(&live_id) {
                            Some(old_data) => {
                                // 直播改标题的情况
                                if live_data.title != old_data.title {
                                    let data_tx = old_data.data_tx.clone();
                                    send_message(
                                        &data_tx,
                                        &live_id,
                                        liver_uid,
                                        LiveData::Title(Title::new(
                                            live_data.live_id,
                                            live_data.title.clone(),
                                        )),
                                    );
                                    let _ = lives.insert(
                                        live_id,
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
                                let _ = tokio::spawn(danmaku(
                                    data_tx.clone(),
                                    live_data,
                                    info.user,
                                    liver_uid,
                                ));
                                let live_id_ = live_id.clone();
                                let _ = tokio::task::spawn_blocking(move || {
                                    save_data(data_rx, live_id_, liver_uid)
                                });
                                let _ = lives.insert(live_id, LiveMapData { title, data_tx });
                            }
                        }
                    } else {
                        log::warn!("[{}] there is no live data in live info", liver_uid);
                    }
                }
                for live_id in all_lives {
                    // 直播结束获取直播总结
                    if !new_all_lives.contains(&live_id) && !all_summaries.contains(&live_id) {
                        //println!("new_all_lives {:?}", new_all_lives);
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
                    log::error!("live ID {} wasn't in lives", live_id);
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

async fn gift(live_id: &str, liver_uid: i64) -> Result<()> {
    let api_client = API_CLIENT_FACTORY.build().await.context(format!(
        "[{}] [{}] failed to build AcFun API client",
        live_id, liver_uid
    ))?;
    let gift_tx = GIFT_TX.get().expect("failed to get GIFT_TX");
    let list = api_client.get_gift_list(live_id).await.context(format!(
        "[{}] [{}] failed to get gift list",
        live_id, liver_uid
    ))?;
    send_message(gift_tx, live_id, liver_uid, list.data.gift_list);

    Ok(())
}

async fn all_summary(live_id: String) -> Result<()> {
    let all_lives_tx = ALL_LIVES_TX.get().expect("failed to get ALL_LIVE_TX");
    let api_client = API_CLIENT_FACTORY
        .build()
        .await
        .context(format!("[{}] failed to build AcFun API client", live_id))?;
    // 为了获取可能被隐藏的直播间的直播总结
    loop {
        let summary = api_client
            .get_summary(&live_id)
            .await
            .context(format!("[{}] failed to get summary", live_id))?;
        time::sleep(SUMMARY_WAIT).await;
        let new_summary = api_client
            .get_summary(&live_id)
            .await
            .context(format!("[{}] failed to get summary", live_id))?;
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

async fn danmaku(
    data_tx: mpsc::UnboundedSender<LiveData>,
    live_data: ApiLiveData,
    liver_info: UserInfo,
    liver_uid: i64,
) {
    let live_tx = LIVE_TX.get().expect("failed to get LIVE_TX");
    let api_client = match API_CLIENT_FACTORY.build().await {
        Ok(client) => client,
        Err(e) => {
            log::error!(
                "[{}] [{}] failed to build AcFun api client: {}",
                live_data.live_id,
                liver_uid,
                e
            );
            send_message(
                live_tx,
                &live_data.live_id,
                liver_uid,
                LiveMessage::StopDanmaku(live_data.live_id.clone()),
            );
            return;
        }
    };
    let live_id = live_data.live_id.clone();
    // 获取直播信息
    let api_client_ = api_client.clone();
    let data_tx_ = data_tx.clone();
    let _ = tokio::spawn(async move {
        let api_client = api_client_;
        let data_tx = data_tx_;
        run_thrice("live_info()", || {
            live_info(
                &api_client,
                &data_tx,
                live_data.clone(),
                liver_info.clone(),
                liver_uid,
            )
        })
        .await;
    });
    match DanmakuClient::from_api_client(&api_client, liver_uid).await {
        Ok(mut client) => {
            if client.live_id() != live_id {
                log::warn!(
                    "[{}] different live id, live data: {}, danmaku client: {}",
                    liver_uid,
                    live_id,
                    client.live_id()
                );
                send_message(
                    live_tx,
                    &live_id,
                    liver_uid,
                    LiveMessage::StopDanmaku(live_id.clone()),
                );
                return;
            }
            log::info!("[{}] [{}] start getting danmaku", live_id, liver_uid);
            // 获取弹幕
            loop {
                match time::timeout(TIMEOUT, client.next()).await {
                    Ok(option) => match option {
                        Some(result) => match result {
                            Ok(msg) => match msg {
                                Danmaku::ActionSignal(signals) => {
                                    action(&data_tx, signals, live_id.clone(), liver_uid).await;
                                }
                                Danmaku::StateSignal(signals) => {
                                    state(&data_tx, signals, live_id.clone(), liver_uid).await;
                                }
                                Danmaku::NotifySignal(signals) => {
                                    notify(&data_tx, signals, live_id.clone(), liver_uid).await;
                                }
                            },
                            Err(e) => {
                                log::warn!(
                                    "[{}] [{}] getting danmaku error: {}",
                                    live_id,
                                    liver_uid,
                                    e
                                );
                                break;
                            }
                        },
                        None => break,
                    },
                    Err(_) => {
                        log::warn!("[{}] [{}] danmaku client timeout", live_id, liver_uid);
                        break;
                    }
                }
            }
            if let Err(e) = client.close().await {
                log::error!(
                    "[{}] [{}] failed to close WebSocket connection: {}",
                    live_id,
                    liver_uid,
                    e
                );
            }
            log::info!("[{}] [{}] stop getting danmaku", live_id, liver_uid);
        }
        Err(e) => {
            log::warn!(
                "[{}] [{}] failed to build AcFun danmaku client: {}",
                live_id,
                liver_uid,
                e
            );
        }
    }
    // 获取直播总结
    let live_id_ = live_id.clone();
    let _ = tokio::spawn(async move {
        let api_client = api_client;
        let data_tx = data_tx;
        let live_id = live_id_;
        run_thrice("summary()", || {
            summary(&api_client, &data_tx, &live_id, liver_uid)
        })
        .await;
    });
    send_message(
        live_tx,
        &live_id,
        liver_uid,
        LiveMessage::StopDanmaku(live_id.clone()),
    );
}

async fn live_info(
    api_client: &DefaultApiClient,
    data_tx: &mpsc::UnboundedSender<LiveData>,
    live_data: ApiLiveData,
    liver_info: UserInfo,
    liver_uid: i64,
) -> Result<()> {
    let mut medal_name = None;
    let mut medal_count = None;
    // 获取主播的守护徽章信息
    if live_data.has_fans_club {
        let list = api_client
            .get_medal_rank_list(liver_uid)
            .await
            .context(format!(
                "[{}] [{}] failed to get medal rank list",
                live_data.live_id, liver_uid
            ))?;
        medal_name = Some(list.club_name);
        medal_count = Some(list.fans_total_count);
    }
    let live_id = live_data.live_id.clone();
    let title = live_data.title.clone();
    let live_info = LiveInfo::new(liver_uid, live_data);
    let liver_info = LiverInfo::new(&live_info, liver_info, medal_name, medal_count);
    let title = Title {
        live_id: live_id.clone(),
        save_time: liver_info.save_time,
        title,
    };
    send_message(data_tx, &live_id, liver_uid, LiveData::LiveInfo(live_info));
    send_message(
        data_tx,
        &live_id,
        liver_uid,
        LiveData::LiverInfo(liver_info),
    );
    send_message(data_tx, &live_id, liver_uid, LiveData::Title(title));

    Ok(())
}

async fn summary(
    api_client: &DefaultApiClient,
    data_tx: &mpsc::UnboundedSender<LiveData>,
    live_id: &str,
    liver_uid: i64,
) -> Result<()> {
    // 获取直播总结
    let summary = api_client.get_summary(live_id).await.context(format!(
        "[{}] [{}] failed to get summary",
        live_id, liver_uid
    ))?;
    send_message(
        data_tx,
        live_id,
        liver_uid,
        LiveData::Summary(Summary::new(live_id.to_string(), summary)),
    );
    time::sleep(SUMMARY_WAIT).await;
    // 获取主播的直播信息
    let info = api_client
        .get_user_live_info(liver_uid)
        .await
        .context(format!(
            "[{}] [{}] failed to get user live info",
            live_id, liver_uid
        ))?;
    let fans_count = Some(info.user.fan_count_value);
    if let Some(data) = &info.live_data {
        // 检查是否意外结束获取弹幕
        if data.live_id == live_id {
            let live_tx = LIVE_TX.get().expect("failed to get LIVE_TX");
            send_message(
                live_tx,
                live_id,
                liver_uid,
                LiveMessage::LiveList(vec![info]),
            );
        }
    }
    let mut medal_name = None;
    let mut medal_count = None;
    // 获取主播的守护徽章信息
    let list = api_client
        .get_medal_rank_list(liver_uid)
        .await
        .context(format!(
            "[{}] [{}] failed to get medal rank list",
            live_id, liver_uid
        ))?;
    if list.has_fans_club {
        // 主播可能在直播中拥有或改变守护徽章
        medal_name = Some(list.club_name);
        medal_count = Some(list.fans_total_count);
    }
    send_message(
        data_tx,
        live_id,
        liver_uid,
        LiveData::UpdateCount(live_id.to_string(), fans_count, medal_name, medal_count),
    );
    // 发送停止sql运行的消息
    send_message(data_tx, live_id, liver_uid, LiveData::Stop);

    Ok(())
}

async fn action(
    data_tx: &mpsc::UnboundedSender<LiveData>,
    signals: Vec<ActionSignal>,
    live_id: String,
    liver_uid: i64,
) {
    for signal in signals {
        match signal {
            ActionSignal::Comment(comment) => {
                send_message(
                    data_tx,
                    &live_id,
                    liver_uid,
                    LiveData::Comment(Comment::new(live_id.clone(), comment)),
                );
            }
            ActionSignal::FollowAuthor(follow) => {
                send_message(
                    data_tx,
                    &live_id,
                    liver_uid,
                    LiveData::Follow(Follow::new(live_id.clone(), follow)),
                );
            }
            ActionSignal::Gift(gift) => {
                send_message(
                    data_tx,
                    &live_id,
                    liver_uid,
                    LiveData::Gift(Gift::new(live_id.clone(), gift)),
                );
            }
            ActionSignal::JoinClub(join_club) => {
                send_message(
                    data_tx,
                    &live_id,
                    liver_uid,
                    LiveData::JoinClub(JoinClub::new(live_id.clone(), join_club)),
                );
            }
            _ => {}
        }
    }
}

async fn state(
    data_tx: &mpsc::UnboundedSender<LiveData>,
    signals: Vec<StateSignal>,
    live_id: String,
    liver_uid: i64,
) {
    for signal in signals {
        match signal {
            StateSignal::AcFunDisplayInfo(info) => {
                send_message(
                    data_tx,
                    &live_id,
                    liver_uid,
                    LiveData::Banana(Some(info.banana_count)),
                );
            }
            StateSignal::DisplayInfo(info) => {
                send_message(data_tx, &live_id, liver_uid, LiveData::WatchingCount(info));
            }
            StateSignal::RedpackList(list) => {
                send_message(data_tx, &live_id, liver_uid, LiveData::Redpack(list));
            }
            StateSignal::ChatCall(call) => {
                send_message(
                    data_tx,
                    &live_id,
                    liver_uid,
                    LiveData::ChatCall(call.into()),
                );
            }
            StateSignal::ChatReady(ready) => {
                send_message(
                    data_tx,
                    &live_id,
                    liver_uid,
                    LiveData::ChatReady(ChatReady::new(live_id.clone(), ready)),
                );
            }
            StateSignal::ChatEnd(end) => {
                send_message(
                    data_tx,
                    &live_id,
                    liver_uid,
                    LiveData::ChatEnd(ChatEnd::new(live_id.clone(), end)),
                );
            }
            StateSignal::AuthorChatCall(call) => {
                send_message(
                    data_tx,
                    &live_id,
                    liver_uid,
                    LiveData::AuthorChatCall(AuthorChatCall::new(live_id.clone(), call)),
                );
            }
            StateSignal::AuthorChatReady(ready) => {
                send_message(
                    data_tx,
                    &live_id,
                    liver_uid,
                    LiveData::AuthorChatReady(AuthorChatReady::new(live_id.clone(), ready)),
                );
            }
            StateSignal::AuthorChatEnd(end) => {
                send_message(
                    data_tx,
                    &live_id,
                    liver_uid,
                    LiveData::AuthorChatEnd(AuthorChatEnd::new(live_id.clone(), end)),
                );
            }
            StateSignal::AuthorChatChangeSoundConfig(config) => {
                send_message(
                    data_tx,
                    &live_id,
                    liver_uid,
                    LiveData::AuthorChatChangeSoundConfig(AuthorChatChangeSoundConfig::new(
                        live_id.clone(),
                        config,
                    )),
                );
            }
            _ => {}
        }
    }
}

async fn notify(
    data_tx: &mpsc::UnboundedSender<LiveData>,
    signals: Vec<NotifySignal>,
    live_id: String,
    liver_uid: i64,
) {
    for signal in signals {
        if let NotifySignal::ViolationAlert(alert) = signal {
            send_message(
                data_tx,
                &live_id,
                liver_uid,
                LiveData::ViolationAlert(ViolationAlert::new(live_id.clone(), alert)),
            );
        }
    }
}
