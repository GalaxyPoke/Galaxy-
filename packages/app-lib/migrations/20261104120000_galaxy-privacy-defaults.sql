-- Galaxy宝可梦: no Discord Rich Presence, telemetry or personalised ads by default.
UPDATE settings
SET discord_rpc = FALSE,
    telemetry = FALSE,
    telemetry_consent_version = 0,
    personalized_ads = FALSE;
