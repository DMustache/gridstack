CREATE TABLE public.identity_signing_keys (
    id UUID PRIMARY KEY DEFAULT public.uuid_generate_v4(),
    usage VARCHAR NOT NULL,
    key_id VARCHAR,
    public_key VARCHAR NOT NULL,
    private_key VARCHAR,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT identity_signing_keys_usage_check CHECK (usage IN ('long_term', 'ephemeral_invite')),
    CONSTRAINT identity_signing_keys_key_id_unique UNIQUE (key_id)
);

CREATE INDEX identity_signing_keys_public_key_idx
    ON public.identity_signing_keys(public_key);

CREATE INDEX identity_signing_keys_usage_expires_at_idx
    ON public.identity_signing_keys(usage, expires_at);

CREATE TABLE public.identity_third_party_invites (
    token VARCHAR PRIMARY KEY,
    medium VARCHAR NOT NULL,
    address VARCHAR NOT NULL,
    room_id VARCHAR NOT NULL,
    sender VARCHAR NOT NULL,
    display_name VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ
);

CREATE INDEX identity_third_party_invites_expires_at_idx
    ON public.identity_third_party_invites(expires_at);
