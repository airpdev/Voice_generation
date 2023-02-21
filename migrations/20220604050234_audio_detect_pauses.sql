-- Add migration script here
CREATE TABLE "audio_detect_pauses" (
    "id" uuid DEFAULT uuid_generate_v4 (),
    "s3_path" TEXT NOT NULL,
    "pauses" TEXT NOT NULL,
    "created_at" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY ("id")
);