define namespace if not exists chat_app;

use ns chat_app;

define table if not exists users schemafull;
define field username on table users type string;
define field created_at on table users type datetime default time::now();
define field phone_number on table users type option<string>;
define field email_addr on table users type option<string>;
define field user_id on table users type uuid;

define table if not exists channels schemafull;
define field channel_name on table channels type string;
define field owner on table channels type option<record<users>> reference;
define field members on table channels type array<record<users>> reference;
define field created_at on table channels type datetime default time::now();
