import type { SettingsSnapshot } from "../../../generated/SettingsSnapshot";
import type { SetupStatus } from "../../../generated/SetupStatus";

// What every step gets: what is saved so far, and where to go next.
export type StepProps = {
  saved: SettingsSnapshot;
  onSaved: (status: SetupStatus) => void;
  onBack?: () => void;
};
