---
tags: [requirements]
modules: [tax]
---
# Requirements: Tax (TAX)

The app is **universal**: each shop enters its own tax rules. Module: [[Module - tax]]. Calculation rules: [[Money Tax and Rounding]], [[ADR-008 Tax model]].

### TAX-01 Tax label
- The shop sets the label used everywhere (UI, receipts, reports): "VAT", "GST", "Sales Tax", "TVA", …

### TAX-02 Named rates
- Admins create rates with a name and a percentage with up to 2 decimals (0.00–100.00). Examples: "Standard 15%", "Reduced 7.5%", "Exempt 0%"
- Each product has exactly one rate

### TAX-03 Rate components (display only)
- A rate can be split into named components that add up to it, e.g. "GST 18%" = "CGST 9%" + "SGST 9%"
- Receipts and the tax report show the components. The calculation uses the total rate, and the tax amount is split across components (the remainder goes to the last component)

### TAX-04 Prices include or exclude tax
- Chosen in the setup wizard for the whole shop
- **Inclusive**: the shelf price is what the customer pays, and tax is taken out of it
- **Exclusive**: tax is added on top of the shelf price
- Changing the mode after sales exist asks for confirmation and affects **only future** sales

### TAX-05 Calculation (each row is a unit test)
| Mode | Line amount | Rate | Net | Tax | Gross |
|---|---|---|---|---|---|
| Inclusive | 115.00 | 15% | 100.00 | 15.00 | 115.00 |
| Inclusive | 100.00 | 15% | 86.96 | 13.04 | 100.00 |
| Exclusive | 100.00 | 15% | 100.00 | 15.00 | 115.00 |
| Exclusive | 99.99 | 7.5% | 99.99 | 7.50 | 107.49 |
| Either | 50.00 | 0% | 50.00 | 0.00 | 50.00 |
- Tax is rounded **per line**, half away from zero, to the currency's minor unit

### TAX-06 Discounts reduce taxable amounts
- Promotion, manual and invoice discounts are applied **before** tax. Invoice discounts are spread across lines in proportion to their amounts

### TAX-07 Rate changes
- Changing a rate's percentage or a product's rate affects only future sales. Past sales keep their stored rate and amounts

### TAX-08 Receipt
- Shows the tax registration number (if set), a tax summary per rate/component (net, tax), and "Prices include <label>" in inclusive mode

### TAX-09 Tax report
- For a date range: per rate and component, net sales, tax collected, returns, and net tax. Exportable ([[Req - Reports]])

### TAX-10 Rates in use
- A rate used by any product or sale can't be deleted, only deactivated (it then can't be chosen for new products)
