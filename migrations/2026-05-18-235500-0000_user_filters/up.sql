CREATE TABLE public.user_filters (
    id BIGSERIAL PRIMARY KEY,
    user_id VARCHAR NOT NULL REFERENCES public.users(user_id) ON DELETE CASCADE,
    filter_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX user_filters_user_id_idx ON public.user_filters(user_id);
