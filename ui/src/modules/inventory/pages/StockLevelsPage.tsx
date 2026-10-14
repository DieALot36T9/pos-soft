import {
  AlertTriangle,
  CheckCircle,
  ChevronLeft,
  ChevronRight,
  Clock,
  FolderOpen,
  ShieldCheck,
  TrendingDown,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import type { Category } from "../../../generated/Category";
import type { MismatchView } from "../../../generated/MismatchView";
import type { StockList } from "../../../generated/StockList";
import type { StockRow } from "../../../generated/StockRow";
import type { StockShow } from "../../../generated/StockShow";
import { errorMessage } from "../../../shared/bridge";
import { formatMoney } from "../../../shared/format/money";
import { formatQty } from "../../../shared/format/qty";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Badge } from "../../../shared/ui/Badge";
import { Field } from "../../../shared/ui/Field";
import { IconButton } from "../../../shared/ui/IconButton";
import { SelectField } from "../../../shared/ui/SelectField";
import { listCategories, listStock, verifyStock } from "../api";
import { StockProductPage } from "./StockProductPage";

const PAGE_SIZE = 50;

type StockLevelsPageProps = {
  canAdjust: boolean;
};

export function StockLevelsPage({ canAdjust }: StockLevelsPageProps) {
  const { t } = useTranslation("inventory");
  const { t: tErrors } = useTranslation("errors");
  const [categories, setCategories] = useState<Category[]>([]);
  const [list, setList] = useState<StockList | null>(null);
  const [text, setText] = useState("");
  const [categoryId, setCategoryId] = useState("");
  const [show, setShow] = useState<StockShow>("all");
  const [offset, setOffset] = useState(0);
  const [open, setOpen] = useState<string | null>(null);
  const [mismatches, setMismatches] = useState<MismatchView[] | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const asked = useRef(0);

  const loadOnce = () => {
    listCategories()
      .then(setCategories)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps -- load once when the page opens.
  useEffect(loadOnce, []);

  const load = () => {
    asked.current += 1;
    const question = asked.current;
    listStock(
      {
        text: text.trim() === "" ? null : text,
        category_id: categoryId === "" ? null : categoryId,
        show,
      },
      { offset, limit: PAGE_SIZE },
    )
      .then((found) => {
        if (question === asked.current) {
          setList(found);
        }
      })
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps -- reload when a filter changes.
  useEffect(load, [text, categoryId, show, offset, open]);

  const filterBy = (apply: () => void) => {
    apply();
    setOffset(0);
  };

  const verify = () => {
    verifyStock()
      .then(setMismatches)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };

  if (open !== null) {
    return <StockProductPage productId={open} canAdjust={canAdjust} onBack={() => setOpen(null)} />;
  }

  return (
    <section aria-labelledby="stock-heading" className="flex flex-col gap-4">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h2 id="stock-heading" className="text-[28px] font-semibold">
            {t("levels.title")}
          </h2>
          <p className="text-text-muted">{t("levels.intro")}</p>
        </div>
        {canAdjust ? (
          <ActionButton
            colour="blue"
            icon={ShieldCheck}
            label={t("levels.verify.label")}
            tip={t("levels.verify.tip")}
            onClick={verify}
          />
        ) : null}
      </div>
      {mismatches !== null && mismatches.length === 0 ? (
        <Alert kind="done">{t("levels.verify.fine")}</Alert>
      ) : null}
      {mismatches !== null && mismatches.length > 0 ? (
        <Alert kind="problem">
          <p>{t("levels.verify.found")}</p>
          <ul className="list-disc pl-5">
            {mismatches.map((row) => (
              <li key={`${row.product_id}-${row.batch_id ?? ""}`}>
                {t("levels.verify.row", {
                  name: row.name,
                  batch:
                    row.batch_id === null
                      ? ""
                      : t("levels.verify.batch", { batch: row.batch_no ?? row.batch_id }),
                  cached: formatQty(row.cached, 3),
                  ledger: formatQty(row.ledger_sum, 3),
                })}
              </li>
            ))}
          </ul>
        </Alert>
      ) : null}

      <div className="rounded-ui border border-border bg-panel p-4 grid grid-cols-1 md:grid-cols-3 gap-3">
        <Field
          label={t("levels.search.label")}
          tip={t("levels.search.tip")}
          value={text}
          onChange={(typed) => filterBy(() => setText(typed))}
        />
        <SelectField
          label={t("levels.category.label")}
          tip={t("levels.category.tip")}
          value={categoryId}
          options={[
            { value: "", label: t("levels.anyCategory") },
            ...categories.map((each) => ({ value: each.id, label: each.name })),
          ]}
          onChange={(picked) => filterBy(() => setCategoryId(picked))}
        />
        <SelectField<StockShow>
          label={t("levels.show.label")}
          tip={t("levels.show.tip")}
          value={show}
          options={(["all", "low", "negative", "expiring"] as const).map((each) => ({
            value: each,
            label: t(`levels.show.${each}`),
          }))}
          onChange={(picked) => filterBy(() => setShow(picked))}
        />
      </div>

      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      {list === null ? null : list.rows.length === 0 ? (
        <p className="rounded-ui border border-border bg-panel p-6 text-center text-text-muted">
          {t("levels.table.empty")}
        </p>
      ) : (
        <table className="w-full border-collapse rounded-ui border border-border bg-panel">
          <thead>
            <tr className="border-b border-border bg-bg text-left text-sm font-bold">
              <th className="p-2.5">{t("levels.table.name")}</th>
              <th className="p-2.5">{t("levels.table.sku")}</th>
              <th className="p-2.5">{t("levels.table.category")}</th>
              <th className="p-2.5 text-right">{t("levels.table.onHand")}</th>
              <th className="p-2.5 text-right">{t("levels.table.reorderLevel")}</th>
              <th className="p-2.5 text-right">{t("levels.table.value")}</th>
              <th className="p-2.5">{t("levels.table.status")}</th>
              <th className="p-2.5">{t("levels.table.actions")}</th>
            </tr>
          </thead>
          <tbody>
            {list.rows.map((row) => (
              <tr
                key={row.product_id}
                onDoubleClick={() => setOpen(row.product_id)}
                className="border-b border-border hover:bg-blue-soft/30"
              >
                <td className="p-2.5 font-semibold">{row.name}</td>
                <td className="p-2.5 font-mono text-sm text-text-muted">{row.sku}</td>
                <td className="p-2.5">{row.category_name}</td>
                <td
                  className={
                    "p-2.5 text-right tabular-nums " +
                    (row.negative ? "font-bold text-red" : "font-semibold")
                  }
                >
                  {t("qtyUnit", {
                    qty: formatQty(row.on_hand, row.unit_decimals),
                    unit: row.unit_symbol,
                  })}
                </td>
                <td className="p-2.5 text-right tabular-nums text-text-muted">
                  {row.reorder_level === null
                    ? ""
                    : formatQty(row.reorder_level, row.unit_decimals)}
                </td>
                <td className="p-2.5 text-right font-semibold tabular-nums">
                  {formatMoney(row.value, list.currency)}
                </td>
                <td className="p-2.5">
                  <StatusBadges row={row} />
                </td>
                <td className="p-2.5">
                  <IconButton
                    icon={FolderOpen}
                    colour="blue"
                    tip={`${t("levels.open.tip")} ${row.name}`}
                    onClick={() => setOpen(row.product_id)}
                  />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      {list ? (
        <div className="flex items-center justify-between">
          <ActionButton
            colour="grey"
            icon={ChevronLeft}
            label={t("levels.previous.label")}
            tip={t("levels.previous.tip")}
            disabledReason={offset === 0 ? t("levels.previous.none") : undefined}
            onClick={() => setOffset(Math.max(0, offset - PAGE_SIZE))}
          />
          <span className="text-sm font-semibold text-text-muted tabular-nums">
            {t("levels.count", {
              shown: list.rows.length,
              total: list.total,
              value: formatMoney(list.total_value, list.currency),
            })}
          </span>
          <ActionButton
            colour="grey"
            icon={ChevronRight}
            label={t("levels.next.label")}
            tip={t("levels.next.tip")}
            disabledReason={offset + PAGE_SIZE >= list.total ? t("levels.next.none") : undefined}
            onClick={() => setOffset(offset + PAGE_SIZE)}
          />
        </div>
      ) : null}
    </section>
  );
}

function StatusBadges({ row }: { row: StockRow }) {
  const { t } = useTranslation("inventory");
  if (!row.low && !row.negative && !row.expiring) {
    return (
      <Badge
        colour="green"
        icon={CheckCircle}
        word={t("levels.badge.fine")}
        tip={t("levels.badge.fineTip")}
      />
    );
  }
  return (
    <span className="flex flex-wrap gap-1">
      {row.negative ? (
        <Badge
          colour="red"
          icon={AlertTriangle}
          word={t("levels.badge.negative")}
          tip={t("levels.badge.negativeTip")}
        />
      ) : null}
      {row.low ? (
        <Badge
          colour="amber"
          icon={TrendingDown}
          word={t("levels.badge.low")}
          tip={t("levels.badge.lowTip")}
        />
      ) : null}
      {row.expiring ? (
        <Badge
          colour="amber"
          icon={Clock}
          word={t("levels.badge.expiring")}
          tip={t("levels.badge.expiringTip")}
        />
      ) : null}
    </span>
  );
}
