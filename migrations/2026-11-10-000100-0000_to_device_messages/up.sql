-- To-device messages share the room sync stream sequence so that a single
-- integer `next_batch` token covers both streams.
CREATE TABLE public.to_device_messages (
    stream_position BIGINT PRIMARY KEY DEFAULT nextval('public.room_sync_stream_stream_position_seq'),
    recipient_user_id VARCHAR NOT NULL,
    recipient_device_id VARCHAR NOT NULL,
    sender_user_id VARCHAR NOT NULL,
    event_type VARCHAR NOT NULL,
    content JSONB NOT NULL
);

CREATE INDEX to_device_messages_recipient_stream_position_idx
    ON public.to_device_messages(recipient_user_id, recipient_device_id, stream_position);
