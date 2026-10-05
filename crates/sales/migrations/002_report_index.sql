-- Reports add up a year of sale lines at a time (RPT-11: any report over a year within 3 s).
-- This index replaces sales_lines_sale: it still finds a sale's lines by sale_id, and it
-- also holds every column the reports add up, so they don't read the wide line rows
-- (names, tax parts). On the reference data it took a year of lines from about 1.6 s to 0.9 s.
-- Page: docs/05 Modules/Module - sales.md
DROP INDEX sales_lines_sale;
CREATE INDEX sales_lines_report ON sales_lines(
  sale_id, product_id, category_id, brand_id, qty_milli, net_minor, tax_minor,
  promo_discount_minor, tier_discount_minor, manual_discount_minor, invoice_discount_minor,
  cost_minor, tax_rate_id, tax_bps, price_overridden
);
