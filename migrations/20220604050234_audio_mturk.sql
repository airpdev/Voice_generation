-- Add migration script here
CREATE TABLE "audio_mturk" (
    "id" uuid DEFAULT uuid_generate_v4 (),
    "mturk_id" TEXT NOT NULL,
    "transcript" TEXT NOT NULL,
    "transcript_id" TEXT NOT NULL,
    "file_path" TEXT NOT NULL,
    "duration" TEXT NOT NULL,
    "status" TEXT NOT NULL, -- status: 0->success 1->failed 2->pending
    "created_at" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP, 
    PRIMARY KEY ("id")
);