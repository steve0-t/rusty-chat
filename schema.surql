define namespace if not exists chat_app;

use ns chat_app;

define table if not exists users schemafull;
define field username on table users type string;
define field created_at on table users type datetime default time::now();
define field phone_number on table users type option<string>;
define field email_addr on table users type option<string>;
define field user_id on table users type uuid;

define table if not exists channels schemafull;
define field channe_name on table channels type string;
