import type { SettingsSnapshot } from "../../../generated/SettingsSnapshot";

// What every settings section gets: the saved settings, and a way to share the new ones.
export type SectionProps = {
  saved: SettingsSnapshot;
  onSaved: (snapshot: SettingsSnapshot) => void;
};
