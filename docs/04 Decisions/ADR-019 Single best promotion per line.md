---
tags: [adr]
status: accepted
---
# ADR-019 Single best promotion per line

## Context
Several promotions can match one product (category 10% off and a product buy-2-get-1). Stacking rules get complicated quickly and surprise both shops and customers.

## Decision
- For each product in the cart, calculate every matching promotion on the combined quantity and apply **only the one that gives the largest discount**. Ties go to the promotion created first.
- Tier discounts apply only to lines with no promotion. Manual discounts apply after.
- The engine is a **pure function** with exhaustive unit tests.

## Consequences
- ✅ Predictable, explainable ("best deal applied"), simple to test.
- ⚠️ Mix-and-match across different products (any 3 of these 5 chocolates) isn't supported in v1.0. Multi-buy works per product. A category-level multi-buy can be added later as a new promotion kind.
