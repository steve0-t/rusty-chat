option import;

define namespace if not exists chat_app;
use ns chat_app;

define database if not exists production;
use db production;

define table if not exists users schemafull;
define field username on table users type string;
define field password on table users type string;
define field created_at on table users type datetime default time::now();
define field phone_number on table users type option<string>;
define field email_addr on table users type option<string>;

define table if not exists channels schemafull;
define field channel_name on table channels type string;
define field owner on table channels type option<record<users>> reference;
define field members on table channels type array<record<users>> reference;
define field created_at on table channels type datetime default time::now();

define table if not exists messages schemafull;
define field sender on table messages type record<users> reference;
define field channel on table messages type record<channels> reference;
define field sent_at on table messages type datetime default time::now();
define field status on table messages type string;
define field content on table messages type array<string>;

define table if not exists sessions;
define field user_id on table sessions type record<users> reference;
define field token_hash on table sessions type string;
define field created_at on table sessions type datetime;
define field expires_at on table sessions type datetime;
define field last_used_at on table sessions type datetime;
define field revoked_at on table sessions type datetime;
define field device_info on table sessions type string;
