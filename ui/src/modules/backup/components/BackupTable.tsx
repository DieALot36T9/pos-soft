import { CheckCircle, RotateCcw, XCircle } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { BackupRow } from "../../../generated/BackupRow";
import { Badge } from "../../../shared/ui/Badge";
import { IconButton } from "../../../shared/ui/IconButton";

type BackupTableProps = {
  rows: BackupRow[];
  onRestore: (row: BackupRow) => void;
};

const knownReasons = ["auto", "close", "manual", "pre-restore", "pre-upgrade"] as const;

export function BackupTable({ rows, onRestore }: BackupTableProps) {
  const { t } = useTranslation("backup");
  if (rows.length === 0) {
    return (
      <p className="rounded-ui border border-border bg-panel p-6 text-center text-text-muted">
        {t("table.empty")}
      </p>
    );
  }
  const reasonText = (reason: string) => {
    const known = knownReasons.find((each) => each === reason);
    return known ? t(`reasons.${known}`) : t("reasons.other", { reason });
  };
  return (
    <table className="w-full border-collapse rounded-ui border border-border bg-panel">
      <thead>
        <tr className="border-b border-border bg-bg text-left text-sm font-bold">
          <th className="p-2.5">{t("table.takenAt")}</th>
          <th className="p-2.5">{t("table.reason")}</th>
          <th className="p-2.5 text-right">{t("table.size")}</th>
          <th className="p-2.5">{t("table.check")}</th>
          <th className="p-2.5 text-right">{t("table.sales")}</th>
          <th className="p-2.5">{t("table.lastInvoice")}</th>
          <th className="p-2.5">{t("table.actions")}</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((row) => (
          <tr key={row.file} className="border-b border-border hover:bg-blue-soft/30">
            <td className="p-2.5 font-mono text-sm tabular-nums">{row.taken_at}</td>
            <td className="p-2.5 font-medium">{reasonText(row.reason)}</td>
            <td className="p-2.5 text-right tabular-nums text-text-muted">
              {t("table.sizeKb", { kb: Math.ceil(row.size_bytes / 1024) })}
            </td>
            <td className="p-2.5">
              {row.usable ? (
                <Badge
                  colour="green"
                  icon={CheckCircle}
                  word={t("table.usable")}
                  tip={t("table.usableTip")}
                />
              ) : (
                <Badge
                  colour="red"
                  icon={XCircle}
                  word={t("table.unusable")}
                  tip={t("table.unusableTip")}
                />
              )}
            </td>
            <td className="p-2.5 text-right font-semibold tabular-nums">{row.sale_count}</td>
            <td className="p-2.5 font-mono text-sm">{row.last_invoice_no ?? t("table.none")}</td>
            <td className="p-2.5">
              {row.usable ? (
                <IconButton
                  icon={RotateCcw}
                  colour="red"
                  tip={t("restore.tip")}
                  onClick={() => onRestore(row)}
                />
              ) : null}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
