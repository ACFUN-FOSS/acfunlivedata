pub const SEMICOLON: &str = r";";
pub const WHERE: &str = r"
WHERE";
pub const AND: &str = r"
AND";
pub const OR: &str = r"
OR";
pub const LEFT_PARENTHESES: &str = r"
(";
pub const RIGHT_PARENTHESES: &str = r"
)";

pub const LIVE_ID: &str = r"
live_id = ?";
pub const LIVER_UID: &str = r"
liver_uid = ?";
pub const USER_ID: &str = r"
user_id = ?";

pub const START_TIME_START: &str = r"
start_time >= ?";
pub const START_TIME_END: &str = r"
start_time <= ?";
pub const SAVE_TIME_START: &str = r"
save_time >= ?";
pub const SAVE_TIME_END: &str = r"
save_time <= ?";
pub const SEND_TIME_START: &str = r"
send_time >= ?";
pub const SEND_TIME_END: &str = r"
send_time <= ?";
pub const JOIN_TIME_START: &str = r"
join_time >= ?";
pub const JOIN_TIME_END: &str = r"
join_time <= ?";

pub const GIFT_ID: &str = r"
gift_id = ?";
pub const ORDER_SAVE_TIME_DESC: &str = r"
ORDER BY save_time DESC";
//pub const LIMIT_ONE: &str = r"
//LIMIT 1";

pub const SELECT_LIVE: &str = r"SELECT
live_id, liver_uid, nickname, stream_name, start_time, title, live_type_id, live_type_name, live_type_category_id, live_type_category_name, portrait, panoramic, disable_danmaku_show, duration, like_count, watch_count
FROM live";

pub const SELECT_GIFT_INFO: &str = r"SELECT
id, save_time, gift_id, gift_name, ar_live_name, pay_wallet_type, gift_price, webp_pic, png_pic, small_png_pic, allow_batch_send_size_list, can_combo, can_draw, magic_face_id, vup_ar_id, description, redpack_price, corner_marker_text
FROM gift_info";

pub const SELECT_LIVE_INFO: &str = r"SELECT
live_id, liver_uid, stream_name, start_time, cover, live_type_id, live_type_name, live_type_category_id, live_type_category_name, has_fans_club, portrait, panoramic, disable_danmaku_show, paid_show_user_buy_status
FROM live_info";

pub const SELECT_TITLE: &str = r"SELECT
live_id, save_time, title
FROM title";

pub const SELECT_LIVER_INFO: &str = r"SELECT
live_id, save_time, liver_uid, nickname, avatar, avatar_frame, following_count, contribute_count, live_begin_fans_count, live_end_fans_count, signature, verified_text, is_join_up_college, medal_name, live_begin_medal_count, live_end_medal_count
FROM liver_info";

pub const SELECT_SUMMARY: &str = r"SELECT
live_id, save_time, duration, like_count, watch_total_count, watch_online_max_count, banana_count
FROM summary";

pub const SELECT_COMMENT: &str = r"SELECT
live_id, send_time, user_id, nickname, avatar, medal_uper_uid, medal_name, medal_level, manager, content
FROM comment";

pub const SELECT_FOLLOW: &str = r"SELECT
live_id, send_time, user_id, nickname, avatar, medal_uper_uid, medal_name, medal_level, manager
FROM follow";

pub const SELECT_GIFT: &str = r"SELECT
live_id, send_time, user_id, nickname, avatar, medal_uper_uid, medal_name, medal_level, manager, gift_id, count, combo, value, combo_id, slot_display_duration, expire_duration, draw_gift_info
FROM gift";

pub const SELECT_JOIN_CLUB: &str = r"SELECT
live_id, join_time, fans_uid, fans_nickname, uper_uid, uper_nickname
FROM join_club";

pub const SELECT_WATCHING_COUNT: &str = r"SELECT
live_id, save_time, watching_count
FROM watching_count";
