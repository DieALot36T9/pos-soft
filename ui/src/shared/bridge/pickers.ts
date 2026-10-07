import { open, save } from "@tauri-apps/plugin-dialog";

// The fake bridge runs in `pnpm dev:mock` (a normal browser) and in tests.
const useFakeBridge = import.meta.env.MODE === "mock" || import.meta.env.MODE === "test";

// What the fake pickers answer, so screens can be tried without the desktop app.
export const FAKE_BACKUP_FILE = "/fake/backups/pos-20261001-120000-manual.sqlite";
export const FAKE_FOLDER = "/fake/usb-drive";

// Lets the user pick one backup file with the computer's file picker. Null if cancelled.
export async function pickBackupFile(title: string): Promise<string | null> {
  if (useFakeBridge) {
    return FAKE_BACKUP_FILE;
  }
  const picked = await open({
    title,
    multiple: false,
    directory: false,
    filters: [{ name: "Backup", extensions: ["sqlite"] }],
  });
  return typeof picked === "string" ? picked : null;
}

// Lets the user pick a folder with the computer's folder picker. Null if cancelled.
export async function pickFolder(title: string): Promise<string | null> {
  if (useFakeBridge) {
    return FAKE_FOLDER;
  }
  const picked = await open({ title, multiple: false, directory: true });
  return typeof picked === "string" ? picked : null;
}

// Lets the user choose where to save an Excel file. Null if cancelled.
export async function pickSaveFile(title: string, suggestedName: string): Promise<string | null> {
  if (useFakeBridge) {
    return `/fake/exports/${suggestedName}`;
  }
  const picked = await save({
    title,
    defaultPath: suggestedName,
    filters: [{ name: "Excel", extensions: ["xlsx"] }],
  });
  return picked ?? null;
}

export const FAKE_IMAGE_FILE = "/fake/pictures/mango.png";

// Lets the user pick a product picture (PNG, JPEG or WebP, CAT-10). Null if cancelled.
export async function pickImageFile(title: string): Promise<string | null> {
  if (useFakeBridge) {
    return FAKE_IMAGE_FILE;
  }
  const picked = await open({
    title,
    multiple: false,
    directory: false,
    filters: [{ name: "Picture", extensions: ["png", "jpg", "jpeg", "webp"] }],
  });
  return typeof picked === "string" ? picked : null;
}

export const FAKE_IMPORT_FILE = "/fake/imports/products.csv";

// Lets the user pick a .csv or .xlsx file to import (IO-01). Null if cancelled.
export async function pickImportFile(title: string): Promise<string | null> {
  if (useFakeBridge) {
    return FAKE_IMPORT_FILE;
  }
  const picked = await open({
    title,
    multiple: false,
    directory: false,
    filters: [{ name: "CSV or Excel", extensions: ["csv", "xlsx"] }],
  });
  return typeof picked === "string" ? picked : null;
}

// Lets the user choose where to save a CSV or Excel file (IO-03). Null if cancelled.
export async function pickDataFile(
  title: string,
  suggestedName: string,
  format: "csv" | "xlsx",
): Promise<string | null> {
  if (useFakeBridge) {
    return `/fake/exports/${suggestedName}`;
  }
  const picked = await save({
    title,
    defaultPath: suggestedName,
    filters:
      format === "csv"
        ? [{ name: "CSV", extensions: ["csv"] }]
        : [{ name: "Excel", extensions: ["xlsx"] }],
  });
  return picked ?? null;
}
