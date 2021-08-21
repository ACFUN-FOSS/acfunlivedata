use acfunliveapi::response::{
    Gift as ApiGift, GiftPicture, LiveData as ApiLiveData, LiveType as ApiLiveType,
    Summary as ApiSummary, UserInfo as ApiUserInfo,
};
use acfunlivedanmaku::{
    acproto::{
        common_state_signal_current_redpack_list::Redpack as ApiRedpack,
        zt_live_user_identity::ManagerType, AcFunUserInfo as ApiAcFunUserInfo,
        AcfunActionSignalJoinClub, AuthorChatPlayerInfo as ApiAuthorChatPlayerInfo,
        CommonActionSignalComment, CommonActionSignalGift, CommonActionSignalUserFollowAuthor,
        CommonStateSignalAuthorChatCall, CommonStateSignalAuthorChatChangeSoundConfig,
        CommonStateSignalAuthorChatEnd, CommonStateSignalAuthorChatReady,
        CommonStateSignalChatCall, CommonStateSignalChatEnd, CommonStateSignalChatReady,
        ImageCdnNode, ZtLiveUserIdentity, ZtLiveUserInfo,
    },
    danmaku::MedalInfo as ApiMedalInfo,
};
use async_graphql::SimpleObject;
use highway::HighwayHasher;
use serde::{Deserialize, Serialize};
use std::{
    hash::{Hash, Hasher},
    sync::Arc,
};

pub type Manager = bool;
pub type LiveId = Arc<String>;

#[inline]
fn unix_time() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[inline]
fn avatar(image: Vec<ImageCdnNode>) -> Option<String> {
    image.into_iter().next().map(|i| i.url)
}

#[inline]
fn gift_picture(picture: Vec<GiftPicture>) -> Option<String> {
    picture.into_iter().next().map(|i| i.url)
}

#[inline]
fn manager(identity: Option<ZtLiveUserIdentity>) -> Option<bool> {
    identity.map(|i| i.manager_type() == ManagerType::Normal)
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, SimpleObject)]
pub struct LiveInfo {
    pub live_id: LiveId,
    pub liver_uid: i64,
    pub liver_info: Option<LiverInfo>,
    pub stream_name: String,
    pub start_time: i64,
    pub title: Option<Vec<Title>>,
    pub cover: Option<String>,
    pub live_type: Option<LiveType>,
    pub has_fans_club: bool,
    pub portrait: bool,
    pub panoramic: bool,
    pub disable_danmaku_show: bool,
    pub paid_show_user_buy_status: bool,
    pub summary: Option<Summary>,
}

impl LiveInfo {
    #[inline]
    pub fn new(liver_uid: i64, data: ApiLiveData) -> Self {
        Self {
            live_id: Arc::new(data.live_id),
            liver_uid,
            liver_info: None,
            stream_name: data.stream_name,
            start_time: data.create_time,
            title: None,
            cover: data.cover_urls.map(|u| u.into_iter().next()).flatten(),
            live_type: data.live_type.map(Into::into),
            has_fans_club: data.has_fans_club,
            portrait: data.portrait,
            panoramic: data.panoramic,
            disable_danmaku_show: data.disable_danmaku_show,
            paid_show_user_buy_status: data.paid_show_user_buy_status,
            summary: None,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, SimpleObject)]
pub struct LiveType {
    pub id: i32,
    pub name: String,
    pub category_id: i32,
    pub category_name: String,
}

impl From<ApiLiveType> for LiveType {
    #[inline]
    fn from(live_type: ApiLiveType) -> Self {
        Self {
            id: live_type.id,
            name: live_type.name,
            category_id: live_type.category_id,
            category_name: live_type.category_name,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, SimpleObject)]
pub struct Title {
    pub live_id: LiveId,
    pub save_time: i64,
    pub title: Option<String>,
}

impl Title {
    #[inline]
    pub fn new(live_id: LiveId, title: Option<String>) -> Self {
        Self {
            live_id,
            save_time: unix_time(),
            title,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, SimpleObject)]
pub struct LiverInfo {
    pub live_id: LiveId,
    pub save_time: i64,
    pub liver_uid: i64,
    pub nickname: String,
    pub avatar: String,
    pub avatar_frame: String,
    pub following_count: i32,
    pub contribute_count: i32,
    pub live_begin_fans_count: i32,
    pub live_end_fans_count: Option<i32>,
    pub signature: Option<String>,
    pub verified_text: Option<String>,
    pub is_join_up_college: Option<bool>,
    pub medal_name: Option<String>,
    pub live_begin_medal_count: Option<i32>,
    pub live_end_medal_count: Option<i32>,
}

impl LiverInfo {
    #[inline]
    pub fn new(
        live_info: &LiveInfo,
        user_info: ApiUserInfo,
        medal_name: Option<String>,
        medal_count: Option<i32>,
    ) -> Self {
        Self {
            live_id: live_info.live_id.clone(),
            save_time: unix_time(),
            liver_uid: live_info.liver_uid,
            nickname: user_info.name,
            avatar: user_info.head_url,
            avatar_frame: user_info.avatar_frame_mobile_img,
            following_count: user_info.following_count_value,
            contribute_count: user_info.contribute_count_value,
            live_begin_fans_count: user_info.fan_count_value,
            live_end_fans_count: None,
            signature: user_info.signature,
            verified_text: user_info.verified_text,
            is_join_up_college: user_info.is_join_up_college,
            medal_name,
            live_begin_medal_count: medal_count,
            live_end_medal_count: None,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, SimpleObject)]
pub struct Summary {
    pub live_id: LiveId,
    pub save_time: i64,
    pub duration: i64,
    pub like_count: String,
    pub watch_total_count: String,
    pub watch_online_max_count: Option<i32>,
    pub banana_count: Option<String>,
}

impl Summary {
    #[inline]
    pub fn new(live_id: LiveId, summary: ApiSummary) -> Self {
        Self {
            live_id,
            save_time: unix_time(),
            duration: summary.data.live_duration_ms,
            like_count: summary.data.like_count,
            watch_total_count: summary.data.watch_count,
            watch_online_max_count: None,
            banana_count: None,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, SimpleObject)]
pub struct Live {
    pub live_id: LiveId,
    pub liver_uid: i64,
    pub nickname: String,
    pub stream_name: String,
    pub start_time: i64,
    pub title: Option<String>,
    pub live_type: Option<LiveType>,
    pub portrait: Option<bool>,
    pub panoramic: Option<bool>,
    pub disable_danmaku_show: Option<bool>,
    pub duration: Option<i64>,
    pub like_count: Option<String>,
    pub watch_count: Option<String>,
}

impl Live {
    #[inline]
    pub fn new(liver_uid: i64, data: &ApiLiveData, info: &ApiUserInfo) -> Self {
        Self {
            live_id: Arc::new(data.live_id.clone()),
            liver_uid,
            nickname: info.name.clone(),
            stream_name: data.stream_name.clone(),
            start_time: data.create_time,
            title: data.title.clone(),
            live_type: data.live_type.clone().map(Into::into),
            portrait: Some(data.portrait),
            panoramic: Some(data.panoramic),
            disable_danmaku_show: Some(data.disable_danmaku_show),
            duration: None,
            like_count: None,
            watch_count: None,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, SimpleObject)]
pub struct GiftInfo {
    pub id: Option<i64>,
    pub save_time: Option<i64>,
    pub gift_id: i64,
    pub gift_name: String,
    pub ar_live_name: String,
    pub pay_wallet_type: i32,
    pub gift_price: i32,
    pub webp_pic: Option<String>,
    pub png_pic: Option<String>,
    pub small_png_pic: Option<String>,
    pub allow_batch_send_size_list: Option<String>,
    pub can_combo: bool,
    pub can_draw: bool,
    pub magic_face_id: i32,
    pub vup_ar_id: i32,
    pub description: String,
    pub redpack_price: i32,
    pub corner_marker_text: String,
}

impl From<ApiGift> for GiftInfo {
    fn from(gift: ApiGift) -> Self {
        let mut gift_info = Self {
            id: None,
            save_time: None,
            gift_id: gift.gift_id,
            gift_name: gift.gift_name,
            ar_live_name: gift.ar_live_name,
            pay_wallet_type: gift.pay_wallet_type,
            gift_price: gift.gift_price,
            webp_pic: gift_picture(gift.webp_pic_list),
            png_pic: gift_picture(gift.png_pic_list),
            small_png_pic: gift_picture(gift.small_png_pic_list),
            allow_batch_send_size_list: serde_json::to_string(&gift.allow_batch_send_size_list)
                .ok(),
            can_combo: gift.can_combo,
            can_draw: gift.can_draw,
            magic_face_id: gift.magic_face_id,
            vup_ar_id: gift.vup_ar_id,
            description: gift.description,
            redpack_price: gift.redpack_price,
            corner_marker_text: gift.corner_marker_text,
        };
        let mut hasher = HighwayHasher::default();
        gift_info.hash(&mut hasher);
        gift_info.id = Some(hasher.finish() as i64);
        gift_info.save_time = Some(unix_time());

        gift_info
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, SimpleObject)]
pub struct MedalInfo {
    pub uper_uid: i64,
    pub name: String,
    pub level: i32,
}

impl MedalInfo {
    #[inline]
    pub fn new(badge: String) -> Option<Self> {
        if badge.is_empty() {
            None
        } else {
            ApiMedalInfo::new(badge).map(Into::into).ok()
        }
    }
}

impl From<ApiMedalInfo> for MedalInfo {
    #[inline]
    fn from(info: ApiMedalInfo) -> Self {
        Self {
            uper_uid: info.uper_id,
            name: info.club_name,
            level: info.level,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, SimpleObject)]
pub struct UserInfo {
    pub user_id: i64,
    pub nickname: String,
    pub avatar: Option<String>,
    pub medal: Option<MedalInfo>,
    pub manager: Option<Manager>,
}

impl From<ZtLiveUserInfo> for UserInfo {
    #[inline]
    fn from(info: ZtLiveUserInfo) -> Self {
        Self {
            user_id: info.user_id,
            nickname: info.nickname,
            avatar: avatar(info.avatar),
            medal: MedalInfo::new(info.badge),
            manager: manager(info.user_identity),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, SimpleObject)]
pub struct Comment {
    pub live_id: LiveId,
    pub send_time: i64,
    pub user_info: Option<UserInfo>,
    pub content: String,
}

impl Comment {
    #[inline]
    pub fn new(live_id: LiveId, comment: CommonActionSignalComment) -> Self {
        Self {
            live_id,
            send_time: comment.send_time_ms,
            user_info: comment.user_info.map(Into::into),
            content: comment.content,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, SimpleObject)]
pub struct Follow {
    pub live_id: LiveId,
    pub send_time: i64,
    pub user_info: Option<UserInfo>,
}

impl Follow {
    #[inline]
    pub fn new(live_id: LiveId, follow: CommonActionSignalUserFollowAuthor) -> Self {
        Self {
            live_id,
            send_time: follow.send_time_ms,
            user_info: follow.user_info.map(Into::into),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, SimpleObject)]
pub struct Gift {
    pub live_id: LiveId,
    pub send_time: i64,
    pub user_info: Option<UserInfo>,
    pub gift_id: i64,
    pub count: i32,
    pub combo: i32,
    pub value: i64,
    pub combo_id: String,
    pub slot_display_duration: i64,
    pub expire_duration: i64,
    pub draw_gift_info: Option<String>,
}

impl Gift {
    #[inline]
    pub fn new(live_id: LiveId, gift: CommonActionSignalGift) -> Self {
        Self {
            live_id,
            send_time: gift.send_time_ms,
            user_info: gift.user.map(Into::into),
            gift_id: gift.gift_id,
            count: gift.count,
            combo: gift.combo,
            value: gift.value,
            combo_id: gift.combo_id,
            slot_display_duration: gift.slot_display_duration_ms,
            expire_duration: gift.expire_duration_ms,
            draw_gift_info: serde_json::to_string(&gift.draw_gift_info).ok(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, SimpleObject)]
pub struct AcFunUserInfo {
    pub user_id: i64,
    pub nickname: String,
}

impl From<ApiAcFunUserInfo> for AcFunUserInfo {
    #[inline]
    fn from(info: ApiAcFunUserInfo) -> Self {
        Self {
            user_id: info.user_id,
            nickname: info.name,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, SimpleObject)]
pub struct JoinClub {
    pub live_id: LiveId,
    pub join_time: i64,
    pub fans_info: Option<AcFunUserInfo>,
    pub uper_info: Option<AcFunUserInfo>,
}

impl JoinClub {
    #[inline]
    pub fn new(live_id: LiveId, join_club: AcfunActionSignalJoinClub) -> Self {
        Self {
            live_id,
            join_time: join_club.join_time_ms,
            fans_info: join_club.fans_info.map(Into::into),
            uper_info: join_club.uper_info.map(Into::into),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, SimpleObject)]
pub struct WatchingCount {
    pub live_id: LiveId,
    pub save_time: i64,
    pub watching_count: Option<i32>,
}

impl WatchingCount {
    #[inline]
    pub fn new(live_id: LiveId, watching_count: Option<i32>) -> Self {
        Self {
            live_id,
            save_time: unix_time(),
            watching_count,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, SimpleObject)]
pub struct Redpack {
    pub redpack_id: String,
    pub live_id: LiveId,
    pub save_time: i64,
    pub sender_info: Option<UserInfo>,
    pub amount: i64,
    pub redpack_biz_unit: String,
    pub get_token_latest_time: i64,
    pub grab_begin_time: i64,
    pub settle_begin_time: i64,
}

impl Redpack {
    #[inline]
    pub fn new(live_id: LiveId, redpack: ApiRedpack) -> Self {
        Self {
            redpack_id: redpack.red_pack_id,
            live_id,
            save_time: unix_time(),
            sender_info: redpack.sender.map(Into::into),
            amount: redpack.redpack_amount,
            redpack_biz_unit: redpack.redpack_biz_unit,
            get_token_latest_time: redpack.get_token_latest_time_ms,
            grab_begin_time: redpack.grab_begin_time_ms,
            settle_begin_time: redpack.settle_begin_time,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, SimpleObject)]
pub struct ChatCall {
    pub chat_id: String,
    pub live_id: String,
    pub call_time: i64,
}

impl From<CommonStateSignalChatCall> for ChatCall {
    #[inline]
    fn from(call: CommonStateSignalChatCall) -> Self {
        Self {
            chat_id: call.chat_id,
            live_id: call.live_id,
            call_time: call.call_timestamp_ms,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, SimpleObject)]
pub struct ChatReady {
    pub chat_id: String,
    pub live_id: LiveId,
    pub save_time: i64,
    pub guest_info: Option<UserInfo>,
    pub media_type: i32,
}

impl ChatReady {
    #[inline]
    pub fn new(live_id: LiveId, ready: CommonStateSignalChatReady) -> Self {
        Self {
            chat_id: ready.chat_id,
            live_id,
            save_time: unix_time(),
            guest_info: ready.guest_user_info.map(Into::into),
            media_type: ready.media_type,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, SimpleObject)]
pub struct ChatEnd {
    pub chat_id: String,
    pub live_id: LiveId,
    pub save_time: i64,
    pub end_type: i32,
}

impl ChatEnd {
    #[inline]
    pub fn new(live_id: LiveId, end: CommonStateSignalChatEnd) -> Self {
        Self {
            chat_id: end.chat_id,
            live_id,
            save_time: unix_time(),
            end_type: end.end_type,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, SimpleObject)]
pub struct AuthorChatPlayerInfo {
    pub player_info: Option<UserInfo>,
    pub live_id: String,
    pub enable_jump_peer_live_room: bool,
}

impl From<ApiAuthorChatPlayerInfo> for AuthorChatPlayerInfo {
    #[inline]
    fn from(info: ApiAuthorChatPlayerInfo) -> Self {
        Self {
            player_info: info.player.map(Into::into),
            live_id: info.live_id,
            enable_jump_peer_live_room: info.enable_jump_peer_live_room,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, SimpleObject)]
pub struct AuthorChatCall {
    pub author_chat_id: String,
    pub live_id: LiveId,
    pub inviter_info: Option<AuthorChatPlayerInfo>,
    pub call_time: i64,
}

impl AuthorChatCall {
    pub fn new(live_id: LiveId, call: CommonStateSignalAuthorChatCall) -> Self {
        Self {
            author_chat_id: call.author_chat_id,
            live_id,
            inviter_info: call.inviter_user_info.map(Into::into),
            call_time: call.call_timestamp_ms,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, SimpleObject)]
pub struct AuthorChatReady {
    pub author_chat_id: String,
    pub live_id: LiveId,
    pub save_time: i64,
    pub inviter_info: Option<AuthorChatPlayerInfo>,
    pub invitee_info: Option<AuthorChatPlayerInfo>,
}

impl AuthorChatReady {
    pub fn new(live_id: LiveId, ready: CommonStateSignalAuthorChatReady) -> Self {
        Self {
            author_chat_id: ready.author_chat_id,
            live_id,
            save_time: unix_time(),
            inviter_info: ready.inviter_user_info.map(Into::into),
            invitee_info: ready.invitee_user_info.map(Into::into),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, SimpleObject)]
pub struct AuthorChatEnd {
    pub author_chat_id: String,
    pub live_id: LiveId,
    pub save_time: i64,
    pub end_type: i32,
    pub end_live_id: String,
}

impl AuthorChatEnd {
    #[inline]
    pub fn new(live_id: LiveId, end: CommonStateSignalAuthorChatEnd) -> Self {
        Self {
            author_chat_id: end.author_chat_id,
            live_id,
            save_time: unix_time(),
            end_type: end.end_type,
            end_live_id: end.end_live_id,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, SimpleObject)]
pub struct AuthorChatChangeSoundConfig {
    pub author_chat_id: String,
    pub live_id: LiveId,
    pub save_time: i64,
    pub sound_config_change_type: i32,
}

impl AuthorChatChangeSoundConfig {
    #[inline]
    pub fn new(live_id: LiveId, config: CommonStateSignalAuthorChatChangeSoundConfig) -> Self {
        Self {
            author_chat_id: config.author_chat_id,
            live_id,
            save_time: unix_time(),
            sound_config_change_type: config.sound_config_change_type,
        }
    }
}
