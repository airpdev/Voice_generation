-- Add migration script here
CREATE TABLE "audio_mturk_users" (
    "id" uuid DEFAULT uuid_generate_v4 (),
    "mturk_id" TEXT NOT NULL,
    "password" TEXT NOT NULL,
    "paypal" TEXT,
    "total_payment" BIGINT NOT NULL,
    "created_at" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP, 
    PRIMARY KEY ("id")
);