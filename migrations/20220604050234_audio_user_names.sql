-- Add migration script here
CREATE TABLE "audio_user_names" (
    "id" uuid DEFAULT uuid_generate_v4 (),
    "user_id" TEXT NOT NULL,
    "file_path" TEXT NOT NULL,
    "voice_code" TEXT NOT NULL,
    "created_at" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    PRIMARY KEY ("id")
);