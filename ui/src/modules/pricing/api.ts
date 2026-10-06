import type { PriceList } from "../../generated/PriceList";
import type { PriceListItem } from "../../generated/PriceListItem";
import type { PriceListItemRow } from "../../generated/PriceListItemRow";
import type { PriceListSave } from "../../generated/PriceListSave";
import type { ProductChoices } from "../../generated/ProductChoices";
import type { Promotion } from "../../generated/Promotion";
import type { PromotionSave } from "../../generated/PromotionSave";
import type { SearchHit } from "../../generated/SearchHit";
import type { Tier } from "../../generated/Tier";
import type { TierSave } from "../../generated/TierSave";
import { call } from "../../shared/bridge";

// Every price list, by name (PRC-02).
export const listPriceLists = () => call<PriceList[]>("price_list_list");
// Adds a price list (no id), or renames or switches one on or off.
export const savePriceList = (input: PriceListSave) => call<string>("price_list_save", { input });
// The prices in one list, with each product's normal price beside it.
export const listItems = (id: string) => call<PriceListItemRow[]>("price_list_items", { id });
// Adds or replaces prices in a list.
export const setItems = (id: string, items: PriceListItem[]) =>
  call<null>("price_list_set_items", { id, items });
// Takes products out of a list: they sell at the normal price again.
export const removeItems = (id: string, productIds: string[]) =>
  call<null>("price_list_remove_items", { id, productIds });
// Every tier, archived ones too (PRC-03).
export const listTiers = () => call<Tier[]>("tier_list");
// Adds a tier (no id) or changes one.
export const saveTier = (input: TierSave) => call<string>("tier_save", { input });
// Archives a tier: it gives nothing from now on, even to customers who have it.
export const archiveTier = (id: string) => call<null>("tier_archive", { id });
// Every promotion, oldest first (PRC-04).
export const listPromotions = () => call<Promotion[]>("promotion_list");
// Adds a promotion (no id) or changes one.
export const savePromotion = (input: PromotionSave) => call<string>("promotion_save", { input });
// The promotion's on/off switch.
export const setPromotionActive = (id: string, active: boolean) =>
  call<null>("promotion_set_active", { id, active });
// Products matching a name, SKU or barcode, to add to a list or promotion.
export const findProducts = (text: string) =>
  call<SearchHit[]>("product_search", { text, limit: 10 });
// The shop's money format, and the categories and brands a promotion can cover.
export const loadChoices = () => call<ProductChoices>("product_choices");
