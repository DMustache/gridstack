CREATE TABLE public.room_receipts (
    stream_position BIGSERIAL PRIMARY KEY,
    room_id VARCHAR NOT NULL REFERENCES public.rooms(room_id) ON DELETE CASCADE,
    user_id VARCHAR NOT NULL,
    receipt_type VARCHAR NOT NULL,
    event_id VARCHAR NOT NULL REFERENCES public.room_events(event_id) ON DELETE CASCADE,
    thread_id VARCHAR NOT NULL DEFAULT '',
    receipt_ts BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX room_receipts_room_stream_position_idx ON public.room_receipts(room_id, stream_position);
CREATE INDEX room_receipts_room_user_stream_position_idx ON public.room_receipts(room_id, user_id, stream_position);
CREATE INDEX room_receipts_room_receipt_type_idx ON public.room_receipts(room_id, receipt_type);

CREATE TABLE public.room_read_markers (
    stream_position BIGSERIAL PRIMARY KEY,
    room_id VARCHAR NOT NULL REFERENCES public.rooms(room_id) ON DELETE CASCADE,
    user_id VARCHAR NOT NULL,
    fully_read_event_id VARCHAR NOT NULL REFERENCES public.room_events(event_id) ON DELETE CASCADE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX room_read_markers_room_user_stream_position_idx ON public.room_read_markers(room_id, user_id, stream_position);
CREATE INDEX room_read_markers_room_user_updated_at_idx ON public.room_read_markers(room_id, user_id, updated_at DESC);
