---
tags: [guide, writing]
---
# Writing Style

The product owner's rule: **every document, comment, piece of code and screen text must be easy to understand.** This page says how.

## For everything
- **Short sentences.** One idea per sentence. If a sentence has more than one comma, split it.
- **Common words.** "use" not "utilise", "start" not "initialise", "check" not "validate" (in prose), "about" not "approximately".
- **Say who does what.** "The app saves the sale" instead of "The sale is persisted".
- **Give an example** whenever a rule could be misread: *"Price for one unit. Example: 120.00"*.
- **Explain a technical word the first time it's used on a page**, or link it to the [[Glossary]].
- **No clever tricks** in code or wording. If it needs a second read, rewrite it.

## Documents (this vault)
- Start each page with one or two sentences saying **what the page is for**.
- Use tables and lists rather than long paragraphs.
- Put the most important thing first.

## Code comments
- Write comments for **why**, in plain English. The code already shows *what*.
- Every public function gets a one-line comment that a non-programmer could follow.
- Good: `// Round to the nearest whole taka, because shops don't use paisa coins.`
- Bad: `// Apply rounding strategy to aggregate.`
- When code follows a rule from the vault, name it: `// Rule CHK-08: scale stickers carry the weight in the barcode.`

## Code itself
- Names say exactly what a thing is: `total_due`, `cash_given`, `change_to_return`, not `amt`, `t2`, `res`.
- Prefer a few more simple lines over one dense line.
- No macros of our own, no deep generic types, no "framework" code. Plain functions and plain structs.
- Each file does one job and is short enough to read in one go.

## Screen text and tooltips
- Buttons: a verb and at most two more words. *"Hold sale"*, *"Pay"*, *"Remove item"*.
- Tooltips: one sentence about what happens, plus the shortcut. *"Put this sale aside and serve the next customer."* `F5`
- Error messages say **what happened and what to do**: *"This barcode is already used by Milk 1L. Use a different barcode or edit Milk 1L."*
- Never blame the user, and never show codes or technical words. Technical details go to the log file.
