define namespace if not exists chat_app;
use ns chat_app;

define database production if not exists;
use db production;

define table if not exists users schemafull;
define field username on table users type string;
define field password on table users type string;
define field created_at on table users type datetime default time::now();
define field phone_number on table users type option<string>;
define field email_addr on table users type option<string>;
define field user_id on table users type uuid;

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
