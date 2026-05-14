CREATE TABLE public.rooms (
    room_id character varying PRIMARY KEY,
    room_version character varying,
    creator_user_id character varying NOT NULL REFERENCES public.users(user_id),
    is_direct boolean,
    name character varying,
    topic text,
    visibility character varying,
    preset character varying,
    created_at timestamp with time zone NOT NULL DEFAULT now(),
    updated_at timestamp with time zone NOT NULL DEFAULT now()
);

CREATE TABLE public.room_aliases (
    alias_localpart character varying PRIMARY KEY,
    room_id character varying NOT NULL REFERENCES public.rooms(room_id) ON DELETE CASCADE,
    created_at timestamp with time zone NOT NULL DEFAULT now()
);

CREATE INDEX room_aliases_room_id_idx ON public.room_aliases(room_id);
CREATE INDEX rooms_creator_user_id_idx ON public.rooms(creator_user_id);

CREATE TABLE public.room_state_events (
    id uuid PRIMARY KEY DEFAULT public.uuid_generate_v4(),
    room_id character varying NOT NULL REFERENCES public.rooms(room_id) ON DELETE CASCADE,
    event_type character varying NOT NULL,
    state_key character varying NOT NULL DEFAULT '',
    content jsonb NOT NULL,
    ordering integer NOT NULL,
    created_at timestamp with time zone NOT NULL DEFAULT now(),
    UNIQUE(room_id, event_type, state_key)
);

CREATE INDEX room_state_events_room_id_ordering_idx
    ON public.room_state_events(room_id, ordering);
