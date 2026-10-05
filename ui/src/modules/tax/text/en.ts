// All screen text for the tax rate screens, in plain English (ADR-022).
export const en = {
  page: {
    title: "Tax rates",
    intro: "Each product has one rate. A rate in use can't be deleted, only made inactive.",
  },
  table: {
    name: "Name",
    percent: "Rate",
    status: "Status",
    actions: "Actions",
    active: "Active",
    inactive: "Inactive",
    activeTip: "Can be chosen for products.",
    inactiveTip: "Kept for past sales, but can't be chosen for new products.",
    empty: "No tax rates yet. Add one below.",
  },
  showInactive: {
    label: "Show inactive rates",
    tip: "Also list rates that were made inactive.",
  },
  edit: { tip: "Change this rate's name or percentage. Past sales keep their old rate." },
  deactivate: {
    tip: "Make this rate inactive. Products and past sales keep it; new products can't choose it.",
  },
  form: {
    addHeading: "Add a rate",
    editHeading: "Change {{name}}",
    name: { label: "Name", tip: "A name for the rate. Example: Reduced 7.5%" },
    percent: { label: "Percent", tip: "The rate, up to 2 decimals. Example: 7.5" },
    percentInvalid: "Type a percentage from 0 to 100, with at most 2 decimals. Example: 7.5",
    save: { label: "Save rate", tip: "Save this rate." },
    cancel: { label: "Cancel", tip: "Stop changing this rate. Nothing is saved." },
    saved: "The rate was saved.",
  },
};

export type TaxText = typeof en;
