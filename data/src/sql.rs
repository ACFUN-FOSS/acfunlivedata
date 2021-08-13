pub const CREATE_LIVE: &str = r"CREATE TABLE IF NOT EXISTS live (
live_id TEXT NOT NULL,
liver_uid INTEGER NOT NULL,
nickname TEXT NOT NULL,
stream_name TEXT NOT NULL,
start_time INTEGER NOT NULL,
title TEXT,
live_type_id INTEGER,
live_type_name TEXT,
live_type_category_id INTEGER,
live_type_category_name TEXT,
portrait INTEGER,
panoramic INTEGER,
disable_danmaku_show INTEGER,
duration INTEGER,
like_count TEXT,
watch_count TEXT
);
CREATE UNIQUE INDEX IF NOT EXISTS live_live_id_index ON live (live_id);
CREATE INDEX IF NOT EXISTS live_liver_uid_index ON live (liver_uid);
CREATE UNIQUE INDEX IF NOT EXISTS live_stream_name_index ON live (stream_name);
CREATE INDEX IF NOT EXISTS live_start_time_index ON live (start_time);";
pub const INSERT_LIVE: &str = r"INSERT OR IGNORE INTO live
(live_id, liver_uid, nickname, stream_name, start_time, title, live_type_id, live_type_name, live_type_category_id, live_type_category_name, portrait, panoramic, disable_danmaku_show, duration, like_count, watch_count)
VALUES (:live_id, :liver_uid, :nickname, :stream_name, :start_time, :title, :live_type_id, :live_type_name, :live_type_category_id, :live_type_category_name, :portrait, :panoramic, :disable_danmaku_show, :duration, :like_count, :watch_count);";
pub const UPDATE_LIVE: &str = r"UPDATE live
SET duration = :duration, like_count = :like_count, watch_count = :watch_count
WHERE live_id = :live_id;";

pub const CREATE_GIFT_INFO: &str = r"CREATE TABLE IF NOT EXISTS gift_info (
id INTEGER NOT NULL,
save_time INTEGER NOT NULL,
gift_id INTEGER NOT NULL,
gift_name TEXT NOT NULL,
ar_live_name TEXT NOT NULL,
pay_wallet_type INTEGER NOT NULL,
gift_price INTEGER NOT NULL,
webp_pic TEXT,
png_pic TEXT,
small_png_pic TEXT,
allow_batch_send_size_list TEXT,
can_combo INTEGER NOT NULL,
can_draw INTEGER NOT NULL,
magic_face_id INTEGER NOT NULL,
vup_ar_id INTEGER NOT NULL,
description TEXT NOT NULL,
redpack_price INTEGER NOT NULL,
corner_marker_text TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS gift_info_id_index ON gift_info (id);
CREATE INDEX IF NOT EXISTS gift_info_gift_id_index ON gift_info (gift_id);";
pub const INSERT_GIFT_INFO: &str = r"INSERT OR IGNORE INTO gift_info
(id, save_time, gift_id, gift_name, ar_live_name, pay_wallet_type, gift_price, webp_pic, png_pic, small_png_pic, allow_batch_send_size_list, can_combo, can_draw, magic_face_id, vup_ar_id, description, redpack_price, corner_marker_text)
VALUES (:id, :save_time, :gift_id, :gift_name, :ar_live_name, :pay_wallet_type, :gift_price, :webp_pic, :png_pic, :small_png_pic, :allow_batch_send_size_list, :can_combo, :can_draw, :magic_face_id, :vup_ar_id, :description, :redpack_price, :corner_marker_text);";

pub const CREATE_LIVE_INFO: &str = r"CREATE TABLE IF NOT EXISTS live_info (
live_id TEXT NOT NULL,
liver_uid INTEGER NOT NULL,
stream_name TEXT NOT NULL,
start_time INTEGER NOT NULL,
cover TEXT,
live_type_id INTEGER,
live_type_name TEXT,
live_type_category_id INTEGER,
live_type_category_name TEXT,
has_fans_club INTEGER NOT NULL,
portrait INTEGER NOT NULL,
panoramic INTEGER NOT NULL,
disable_danmaku_show INTEGER NOT NULL,
paid_show_user_buy_status INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS live_info_live_id_index ON live_info (live_id);
CREATE INDEX IF NOT EXISTS live_info_liver_uid_index ON live_info (liver_uid);
CREATE UNIQUE INDEX IF NOT EXISTS live_info_stream_name_index ON live_info (stream_name);
CREATE UNIQUE INDEX IF NOT EXISTS live_info_start_time_index ON live_info (start_time);";
pub const INSERT_LIVE_INFO: &str = r"INSERT OR IGNORE INTO live_info
(live_id, liver_uid, stream_name, start_time, cover, live_type_id, live_type_name, live_type_category_id, live_type_category_name, has_fans_club, portrait, panoramic, disable_danmaku_show, paid_show_user_buy_status)
VALUES (:live_id, :liver_uid, :stream_name, :start_time, :cover, :live_type_id, :live_type_name, :live_type_category_id, :live_type_category_name, :has_fans_club, :portrait, :panoramic, :disable_danmaku_show, :paid_show_user_buy_status);";

pub const CREATE_TITLE: &str = r"CREATE TABLE IF NOT EXISTS title (
live_id TEXT NOT NULL,
save_time INTEGER NOT NULL,
title TEXT
);
CREATE INDEX IF NOT EXISTS title_live_id_index ON title (live_id);
CREATE UNIQUE INDEX IF NOT EXISTS title_save_time_index ON title (save_time);";
pub const SELECT_TITLE: &str = r"SELECT title FROM title
WHERE live_id = :live_id
ORDER BY save_time DESC
LIMIT 1;";
pub const INSERT_TITLE: &str = r"INSERT INTO title
(live_id, save_time, title)
VALUES (:live_id, :save_time, :title);";

pub const CREATE_LIVER_INFO: &str = r"CREATE TABLE IF NOT EXISTS liver_info (
live_id TEXT NOT NULL,
save_time INTEGER NOT NULL,
liver_uid INTEGER NOT NULL,
nickname TEXT NOT NULL,
avatar TEXT NOT NULL,
avatar_frame TEXT NOT NULL,
following_count INTEGER NOT NULL,
contribute_count INTEGER NOT NULL,
live_begin_fans_count INTEGER NOT NULL,
live_end_fans_count INTEGER,
signature TEXT,
verified_text TEXT,
is_join_up_college INTEGER,
medal_name TEXT,
live_begin_medal_count INTEGER,
live_end_medal_count INTEGER
);
CREATE UNIQUE INDEX IF NOT EXISTS liver_info_live_id_index ON liver_info (live_id);
CREATE UNIQUE INDEX IF NOT EXISTS liver_info_save_time_index ON liver_info (save_time);";
pub const INSERT_LIVER_INFO: &str = r"INSERT OR IGNORE INTO liver_info
(live_id, save_time, liver_uid, nickname, avatar, avatar_frame, following_count, contribute_count, live_begin_fans_count, live_end_fans_count, signature, verified_text, is_join_up_college, medal_name, live_begin_medal_count, live_end_medal_count)
VALUES (:live_id, :save_time, :liver_uid, :nickname, :avatar, :avatar_frame, :following_count, :contribute_count, :live_begin_fans_count, :live_end_fans_count, :signature, :verified_text, :is_join_up_college, :medal_name, :live_begin_medal_count, :live_end_medal_count);";
pub const UPDATE_LIVER_INFO: &str = r"UPDATE liver_info
SET live_end_fans_count = :fans_count, medal_name = :medal_name, live_end_medal_count = :medal_count
WHERE live_id = :live_id;";

pub const CREATE_SUMMARY: &str = r"CREATE TABLE IF NOT EXISTS summary (
live_id TEXT NOT NULL,
save_time INTEGER NOT NULL,
duration INTEGER NOT NULL,
like_count TEXT NOT NULL,
watch_total_count TEXT NOT NULL,
watch_online_max_count INTEGER,
banana_count TEXT
);
CREATE UNIQUE INDEX IF NOT EXISTS summary_live_id_index ON summary (live_id);
CREATE UNIQUE INDEX IF NOT EXISTS summary_save_time_index ON summary (save_time);";
pub const REPLACE_SUMMARY: &str = r"INSERT OR REPLACE INTO summary
(live_id, save_time, duration, like_count, watch_total_count, watch_online_max_count, banana_count)
VALUES (:live_id, :save_time, :duration, :like_count, :watch_total_count, :watch_online_max_count, :banana_count);";

pub const CREATE_COMMENT: &str = r"CREATE TABLE IF NOT EXISTS comment (
live_id TEXT NOT NULL,
send_time INTEGER NOT NULL,
user_id INTEGER,
nickname TEXT,
avatar TEXT,
medal_uper_uid INTEGER,
medal_name TEXT,
medal_level INTEGER,
manager INTEGER,
content TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS comment_live_id_index ON comment (live_id);
CREATE INDEX IF NOT EXISTS comment_send_time_index ON comment (send_time);
CREATE INDEX IF NOT EXISTS comment_user_id_index ON comment (user_id);";
pub const INSERT_COMMENT: &str = r"INSERT INTO comment
(live_id, send_time, user_id, nickname, avatar, medal_uper_uid, medal_name, medal_level, manager, content)
VALUES (:live_id, :send_time, :user_id, :nickname, :avatar, :medal_uper_uid, :medal_name, :medal_level, :manager, :content);";

pub const CREATE_FOLLOW: &str = r"CREATE TABLE IF NOT EXISTS follow (
live_id TEXT NOT NULL,
send_time INTEGER NOT NULL,
user_id INTEGER,
nickname TEXT,
avatar TEXT,
medal_uper_uid INTEGER,
medal_name TEXT,
medal_level INTEGER,
manager INTEGER
);
CREATE INDEX IF NOT EXISTS follow_live_id_index ON follow (live_id);
CREATE INDEX IF NOT EXISTS follow_send_time_index ON follow (send_time);";
pub const INSERT_FOLLOW: &str = r"INSERT INTO follow
(live_id, send_time, user_id, nickname, avatar, medal_uper_uid, medal_name, medal_level, manager)
VALUES (:live_id, :send_time, :user_id, :nickname, :avatar, :medal_uper_uid, :medal_name, :medal_level, :manager);";

pub const CREATE_GIFT: &str = r"CREATE TABLE IF NOT EXISTS gift (
live_id TEXT NOT NULL,
send_time INTEGER NOT NULL,
user_id INTEGER,
nickname TEXT,
avatar TEXT,
medal_uper_uid INTEGER,
medal_name TEXT,
medal_level INTEGER,
manager INTEGER,
gift_id INTEGER NOT NULL,
count INTEGER NOT NULL,
combo INTEGER NOT NULL,
value INTEGER NOT NULL,
combo_id TEXT NOT NULL,
slot_display_duration INTEGER NOT NULL,
expire_duration INTEGER NOT NULL,
draw_gift_info TEXT
);
CREATE INDEX IF NOT EXISTS gift_live_id_index ON gift (live_id);
CREATE INDEX IF NOT EXISTS gift_send_time_index ON gift (send_time);
CREATE INDEX IF NOT EXISTS gift_user_id_index ON gift (user_id);
CREATE INDEX IF NOT EXISTS gift_gift_id_index ON gift (gift_id);";
pub const INSERT_GIFT: &str = r"INSERT INTO gift
(live_id, send_time, user_id, nickname, avatar, medal_uper_uid, medal_name, medal_level, manager, gift_id, count, combo, value, combo_id, slot_display_duration, expire_duration, draw_gift_info)
VALUES (:live_id, :send_time, :user_id, :nickname, :avatar, :medal_uper_uid, :medal_name, :medal_level, :manager, :gift_id, :count, :combo, :value, :combo_id, :slot_display_duration, :expire_duration, :draw_gift_info);";

pub const CREATE_JOIN_CLUB: &str = r"CREATE TABLE IF NOT EXISTS join_club (
live_id TEXT NOT NULL,
join_time INTEGER NOT NULL,
fans_uid INTEGER,
fans_nickname TEXT,
uper_uid INTEGER,
uper_nickname TEXT
);
CREATE INDEX IF NOT EXISTS join_club_live_id_index ON join_club (live_id);
CREATE INDEX IF NOT EXISTS join_club_join_time_index ON join_club (join_time);";
pub const INSERT_JOIN_CLUB: &str = r"INSERT INTO join_club
(live_id, join_time, fans_uid, fans_nickname, uper_uid, uper_nickname)
VALUES (:live_id, :join_time, :fans_uid, :fans_nickname, :uper_uid, :uper_nickname);";

pub const CREATE_WATCHING_COUNT: &str = r"CREATE TABLE IF NOT EXISTS watching_count (
live_id TEXT NOT NULL,
save_time INTEGER NOT NULL,
watching_count INTEGER
);
CREATE INDEX IF NOT EXISTS watching_count_live_id_index ON watching_count (live_id);
CREATE UNIQUE INDEX IF NOT EXISTS watching_count_save_time_index ON watching_count (save_time);";
pub const INSERT_WATCHING_COUNT: &str = r"INSERT INTO watching_count
(live_id, save_time, watching_count)
VALUES (:live_id, :save_time, :watching_count);";

pub const CREATE_REDPACK: &str = r"CREATE TABLE IF NOT EXISTS redpack (
redpack_id TEXT NOT NULL,
live_id TEXT NOT NULL,
save_time INTEGER NOT NULL,
sender_user_id INTEGER,
sender_nickname TEXT,
sender_avatar TEXT,
sender_medal_uper_uid INTEGER,
sender_medal_name TEXT,
sender_medal_level INTEGER,
sender_manager INTEGER,
amount INTEGER NOT NULL,
redpack_biz_unit TEXT NOT NULL,
get_token_latest_time INTEGER NOT NULL,
grab_begin_time INTEGER NOT NULL,
settle_begin_time INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS redpack_redpack_id_index ON redpack (redpack_id);
CREATE INDEX IF NOT EXISTS redpack_live_id_index ON redpack (live_id);
CREATE INDEX IF NOT EXISTS redpack_save_time_index ON redpack (save_time);
CREATE INDEX IF NOT EXISTS redpack_sender_user_id_index ON redpack (sender_user_id);";
pub const INSERT_REDPACK: &str = r"INSERT INTO redpack
(redpack_id, live_id, save_time, sender_user_id, sender_nickname, sender_avatar, sender_medal_uper_uid, sender_medal_name, sender_medal_level, sender_manager, amount, redpack_biz_unit, get_token_latest_time, grab_begin_time, settle_begin_time)
VALUES (:redpack_id, :live_id, :save_time, :sender_user_id, :sender_nickname, :sender_avatar, :sender_medal_uper_uid, :sender_medal_name, :sender_medal_level, :sender_manager, :amount, :redpack_biz_unit, :get_token_latest_time, :grab_begin_time, :settle_begin_time);";

pub const CREATE_CHAT_CALL: &str = r"CREATE TABLE IF NOT EXISTS chat_call (
chat_id TEXT NOT NULL,
live_id TEXT NOT NULL,
call_time INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS chat_call_chat_id_index ON chat_call (chat_id);
CREATE INDEX IF NOT EXISTS chat_call_live_id_index ON chat_call (live_id);
CREATE INDEX IF NOT EXISTS chat_call_call_time_index ON chat_call (call_time);";
pub const INSERT_CHAT_CALL: &str = r"INSERT INTO chat_call
(chat_id, live_id, call_time)
VALUES (:chat_id, :live_id, :call_time);";

pub const CREATE_CHAT_READY: &str = r"CREATE TABLE IF NOT EXISTS chat_ready (
chat_id TEXT NOT NULL,
live_id TEXT NOT NULL,
save_time INTEGER NOT NULL,
guest_user_id INTEGER,
guest_nickname TEXT,
guest_avatar TEXT,
guest_medal_uper_uid INTEGER,
guest_medal_name TEXT,
guest_medal_level INTEGER,
guest_manager INTEGER,
media_type INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS chat_ready_chat_id_index ON chat_ready (chat_id);
CREATE INDEX IF NOT EXISTS chat_ready_live_id_index ON chat_ready (live_id);
CREATE UNIQUE INDEX IF NOT EXISTS chat_ready_save_time_index ON chat_ready (save_time);";
pub const INSERT_CHAT_READY: &str = r"INSERT INTO chat_ready
(chat_id, live_id, save_time, guest_user_id, guest_nickname, guest_avatar, guest_medal_uper_uid, guest_medal_name, guest_medal_level, guest_manager, media_type)
VALUES (:chat_id, :live_id, :save_time, :guest_user_id, :guest_nickname, :guest_avatar, :guest_medal_uper_uid, :guest_medal_name, :guest_medal_level, :guest_manager, :media_type);";

pub const CREATE_CHAT_END: &str = r"CREATE TABLE IF NOT EXISTS chat_end (
chat_id TEXT NOT NULL,
live_id TEXT NOT NULL,
save_time INTEGER NOT NULL,
end_type INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS chat_end_chat_id_index ON chat_end (chat_id);
CREATE INDEX IF NOT EXISTS chat_end_live_id_index ON chat_end (live_id);
CREATE UNIQUE INDEX IF NOT EXISTS chat_end_save_time_index ON chat_end (save_time);";
pub const INSERT_CHAT_END: &str = r"INSERT INTO chat_end
(chat_id, live_id, save_time, end_type)
VALUES (:chat_id, :live_id, :save_time, :end_type);";

pub const CREATE_AUTHOR_CHAT_CALL: &str = r"CREATE TABLE IF NOT EXISTS author_chat_call (
author_chat_id TEXT NOT NULL,
live_id TEXT NOT NULL,
inviter_user_id INTEGER,
inviter_nickname TEXT,
inviter_avatar TEXT,
inviter_medal_uper_uid INTEGER,
inviter_medal_name TEXT,
inviter_medal_level INTEGER,
inviter_manager INTEGER,
inviter_live_id TEXT,
inviter_enable_jump_peer_live_room INTEGER,
call_time INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS author_chat_call_author_chat_id_index ON author_chat_call (author_chat_id);
CREATE INDEX IF NOT EXISTS author_chat_call_live_id_index ON author_chat_call (live_id);
CREATE INDEX IF NOT EXISTS author_chat_call_call_time_index ON author_chat_call (call_time);";
pub const INSERT_AUTHOR_CHAT_CALL: &str = r"INSERT INTO author_chat_call
(author_chat_id, live_id, inviter_user_id, inviter_nickname, inviter_avatar, inviter_medal_uper_uid, inviter_medal_name, inviter_medal_level, inviter_manager, inviter_live_id, inviter_enable_jump_peer_live_room, call_time)
VALUES (:author_chat_id, :live_id, :inviter_user_id, :inviter_nickname, :inviter_avatar, :inviter_medal_uper_uid, :inviter_medal_name, :inviter_medal_level, :inviter_manager, :inviter_live_id, :inviter_enable_jump_peer_live_room, :call_time);";

pub const CREATE_AUTHOR_CHAT_READY: &str = r"CREATE TABLE IF NOT EXISTS author_chat_ready (
author_chat_id TEXT NOT NULL,
live_id TEXT NOT NULL,
save_time INTEGER NOT NULL,
inviter_user_id INTEGER,
inviter_nickname TEXT,
inviter_avatar TEXT,
inviter_medal_uper_uid INTEGER,
inviter_medal_name TEXT,
inviter_medal_level INTEGER,
inviter_manager INTEGER,
inviter_live_id TEXT,
inviter_enable_jump_peer_live_room INTEGER,
invitee_user_id INTEGER,
invitee_nickname TEXT,
invitee_avatar TEXT,
invitee_medal_uper_uid INTEGER,
invitee_medal_name TEXT,
invitee_medal_level INTEGER,
invitee_manager INTEGER,
invitee_live_id TEXT,
invitee_enable_jump_peer_live_room INTEGER
);
CREATE UNIQUE INDEX IF NOT EXISTS author_chat_ready_author_chat_id_index ON author_chat_ready (author_chat_id);
CREATE INDEX IF NOT EXISTS author_chat_ready_live_id_index ON author_chat_ready (live_id);
CREATE UNIQUE INDEX IF NOT EXISTS author_chat_ready_save_time_index ON author_chat_ready (save_time)";
pub const INSERT_AUTHOR_CHAT_READY: &str = r"INSERT INTO author_chat_ready
(author_chat_id, live_id, save_time, inviter_user_id, inviter_nickname, inviter_avatar, inviter_medal_uper_uid, inviter_medal_name, inviter_medal_level, inviter_manager, inviter_live_id, inviter_enable_jump_peer_live_room, invitee_user_id, invitee_nickname, invitee_avatar, invitee_medal_uper_uid, invitee_medal_name, invitee_medal_level, invitee_manager, invitee_live_id, invitee_enable_jump_peer_live_room)
VALUES (:author_chat_id, :live_id, :save_time, :inviter_user_id, :inviter_nickname, :inviter_avatar, :inviter_medal_uper_uid, :inviter_medal_name, :inviter_medal_level, :inviter_manager, :inviter_live_id, :inviter_enable_jump_peer_live_room, :invitee_user_id, :invitee_nickname, :invitee_avatar, :invitee_medal_uper_uid, :invitee_medal_name, :invitee_medal_level, :invitee_manager, :invitee_live_id, :invitee_enable_jump_peer_live_room);";

pub const CREATE_AUTHOR_CHAT_END: &str = r"CREATE TABLE IF NOT EXISTS author_chat_end (
author_chat_id TEXT NOT NULL,
live_id TEXT NOT NULL,
save_time INTEGER NOT NULL,
end_type INTEGER NOT NULL,
end_live_id TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS author_chat_end_author_chat_id_index ON author_chat_end (author_chat_id);
CREATE INDEX IF NOT EXISTS author_chat_end_live_id_index ON author_chat_end (live_id);
CREATE UNIQUE INDEX IF NOT EXISTS author_chat_end_save_time_index ON author_chat_end (save_time);";
pub const INSERT_AUTHOR_CHAT_END: &str = r"INSERT INTO author_chat_end
(author_chat_id, live_id, save_time, end_type, end_live_id)
VALUES (:author_chat_id, :live_id, :save_time, :end_type, :end_live_id);";

pub const CREATE_AUTHOR_CHAT_CHANGE_SOUND_CONFIG: &str = r"CREATE TABLE IF NOT EXISTS author_chat_change_sound_config (
author_chat_id TEXT NOT NULL,
live_id TEXT NOT NULL,
save_time INTEGER NOT NULL,
sound_config_change_type INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS author_chat_change_sound_config_author_chat_id_index ON author_chat_change_sound_config (author_chat_id);
CREATE INDEX IF NOT EXISTS author_chat_change_sound_config_live_id_index ON author_chat_change_sound_config (live_id);
CREATE UNIQUE INDEX IF NOT EXISTS author_chat_change_sound_config_save_time_index ON author_chat_change_sound_config (save_time);";
pub const INSERT_AUTHOR_CHAT_CHANGE_SOUND_CONFIG: &str = r"INSERT INTO author_chat_change_sound_config
(author_chat_id, live_id, save_time, sound_config_change_type)
VALUES (:author_chat_id, :live_id, :save_time, :sound_config_change_type);";

pub const CREATE_VIOLATION_ALERT: &str = r"CREATE TABLE IF NOT EXISTS violation_alert (
live_id TEXT NOT NULL,
save_time INTEGER NOT NULL,
violation_content TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS violation_alert_live_id_index ON violation_alert (live_id);
CREATE UNIQUE INDEX IF NOT EXISTS violation_alert_save_time_index ON violation_alert (save_time);";
pub const INSERT_VIOLATION_ALERT: &str = r"INSERT INTO violation_alert
(live_id, save_time, violation_content)
VALUES (:live_id, :save_time, :violation_content);";
