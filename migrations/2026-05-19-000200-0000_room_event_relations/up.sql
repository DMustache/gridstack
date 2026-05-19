CREATE TABLE public.room_event_relations (
    room_id VARCHAR NOT NULL REFERENCES public.rooms(room_id) ON DELETE CASCADE,
    event_id VARCHAR NOT NULL REFERENCES public.room_events(event_id) ON DELETE CASCADE,
    rel_type VARCHAR NOT NULL,
    related_event_id VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (room_id, event_id, rel_type, related_event_id)
);

CREATE INDEX room_event_relations_room_related_event_idx ON public.room_event_relations(room_id, related_event_id);
CREATE INDEX room_event_relations_room_event_idx ON public.room_event_relations(room_id, event_id);

INSERT INTO public.room_event_relations (room_id, event_id, rel_type, related_event_id)
SELECT
    event.room_id,
    event.event_id,
    event.content_json -> 'm.relates_to' ->> 'rel_type',
    event.content_json -> 'm.relates_to' ->> 'event_id'
FROM public.room_events event
WHERE jsonb_typeof(event.content_json -> 'm.relates_to') = 'object'
  AND COALESCE(event.content_json -> 'm.relates_to' ->> 'rel_type', '') <> ''
  AND COALESCE(event.content_json -> 'm.relates_to' ->> 'event_id', '') <> ''
ON CONFLICT DO NOTHING;
