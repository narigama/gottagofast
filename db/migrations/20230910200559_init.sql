-- migrate:up
create table "fortune" (
    "id" uuid not null default gen_random_uuid(),
    "created_at" timestamptz not null default date_trunc('second', current_timestamp),
    "updated_at" timestamptz not null default date_trunc('second', current_timestamp),

    "content" text not null
);
alter table "fortune" add constraint "fortune_pkey" primary key ("id");

-- migrate:down
drop table "fortune";
