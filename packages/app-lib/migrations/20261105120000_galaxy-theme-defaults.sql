-- Galaxy宝可梦: start in the light theme with the green accent (Modrinth's colours).
-- Only rows still on an earlier default are changed, so a theme or accent the
-- player picked deliberately is kept.
UPDATE settings
SET theme = 'light',
    accent_color = 'green'
WHERE theme = 'dark'
  AND accent_color IN ('pink', 'custom:#b5332a');
