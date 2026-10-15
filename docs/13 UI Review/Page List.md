---
tags: [ui, page-list]
---
# Page List

Every screen and dialog of the app, with its current implementation status in `ui/src/`.

**Progress:** 49 of 50 implemented and overhauled in `ui/src/` (`P35 Sales journal` is the only remaining unbuilt back-office screen).

Statuses: ⬜ Not started · 🟦 In review · 🟨 Changes asked · 🟩 Implemented in `ui/` · ⏸ Later

### Foundations
| ID | Page | Type | Implementation in `ui/src/` | Status |
|---|---|---|---|---|
| P00 | [[P00 App shell and navigation\|App shell & navigation]] | Frame | `app/AppFrame.tsx`, `OfficeFrame.tsx`, `TillFrame.tsx`, `CommandPalette.tsx` | 🟩 Implemented in `ui/` |

### Getting in
| ID | Page | Type | Implementation in `ui/src/` | Status |
|---|---|---|---|---|
| P01 | [[P01 Login\|Login]] | Page | `modules/auth/pages/LoginPage.tsx`, `RecoveryForm.tsx` | 🟩 Implemented in `ui/` |
| P02 | [[P02 Lock screen\|Lock screen]] | Page | `modules/auth/pages/LockScreen.tsx` | 🟩 Implemented in `ui/` |
| P03 | [[P03 Admin approval\|Admin approval]] | Dialog | `shared/approval/ApprovalDialog.tsx` | 🟩 Implemented in `ui/` |

### Till
| ID | Page | Type | Implementation in `ui/src/` | Status |
|---|---|---|---|---|
| P04 | [[P04 Open shift\|Open shift]] | Dialog | `modules/shifts/pages/OpenShiftDialog.tsx` | 🟩 Implemented in `ui/` |
| P05 | [[P05 Checkout\|Checkout]] | Page | `modules/checkout/pages/CheckoutPage.tsx`, `ProductCatalogPane.tsx`, `CartTable.tsx`, `TotalsPanel.tsx` | 🟩 Implemented in `ui/` |
| P06 | [[P06 Line panel: quantity, discount, price\|Line panel: quantity, discount, price]] | Drawer + Dialogs | `modules/checkout/components/LineEditDrawer.tsx`, `QtyDialog.tsx`, `DiscountDialog.tsx`, `PriceDialog.tsx` | 🟩 Implemented in `ui/` |
| P07 | [[P07 Weight entry\|Weight entry]] | Dialog | `modules/checkout/components/WeightDialog.tsx` | 🟩 Implemented in `ui/` |
| P08 | [[P08 Customer at the till\|Customer at the till]] | Dialog | `modules/checkout/components/CustomerDialog.tsx`, `CustomerBar.tsx` | 🟩 Implemented in `ui/` |
| P09 | [[P09 Payment\|Payment]] | Dialog | `modules/checkout/components/PaymentDialog.tsx`, `PaymentRow.tsx` | 🟩 Implemented in `ui/` |
| P10 | [[P10 Sale complete and receipts\|Sale complete & receipts]] | Panel + Dialog | `modules/checkout/components/SaleDoneBar.tsx`, `ReprintDialog.tsx`, `shared/print/PrintProblem.tsx` | 🟩 Implemented in `ui/` |
| P11 | [[P11 Hold and held sales\|Hold & held sales]] | Dialogs | `modules/checkout/components/HoldDialog.tsx`, `HeldListDialog.tsx` | 🟩 Implemented in `ui/` |
| P12 | [[P12 Cancel and void sale\|Cancel & void sale]] | Dialogs | `modules/checkout/components/CancelDialog.tsx`, `VoidDialog.tsx` | 🟩 Implemented in `ui/` |
| P13 | [[P13 Returns and exchange\|Returns & exchange]] | Dialog | `modules/checkout/components/ReturnDialog.tsx`, `ReceiptLines.tsx`, `NoReceiptLines.tsx`, `RefundSummary.tsx` | 🟩 Implemented in `ui/` |
| P14 | [[P14 Till More menu and cash moves\|Till 'More' menu & cash moves]] | Menu + Dialogs | `modules/checkout/components/ButtonBar.tsx`, `modules/shifts/pages/ShiftMenu.tsx`, `CashMoveDialog.tsx` | 🟩 Implemented in `ui/` |
| P15 | [[P15 Close shift\|Close shift]] | Dialog | `modules/shifts/pages/CloseShiftDialog.tsx`, `ClosedSummary.tsx`, `CountForm.tsx` | 🟩 Implemented in `ui/` |

### Back office
| ID | Page | Type | Implementation in `ui/src/` | Status |
|---|---|---|---|---|
| P16 | [[P16 Home dashboard\|Home dashboard]] | Page | `app/HomePage.tsx`, `modules/inventory/components/StockAlertsCard.tsx` | 🟩 Implemented in `ui/` |

### Catalog
| ID | Page | Type | Implementation in `ui/src/` | Status |
|---|---|---|---|---|
| P17 | [[P17 Products list\|Products list]] | Page + Drawer | `modules/catalog/pages/ProductsPage.tsx` | 🟩 Implemented in `ui/` |
| P18 | [[P18 Product editor\|Product editor]] | Page | `modules/catalog/pages/ProductEditor.tsx` (+ 7 tab components) | 🟩 Implemented in `ui/` |
| P19 | [[P19 Categories\|Categories]] | Page | `modules/catalog/pages/CategoriesPage.tsx` | 🟩 Implemented in `ui/` |
| P20 | [[P20 Brands and units\|Brands & units]] | Pages | `modules/catalog/pages/BrandsPage.tsx`, `UnitsPage.tsx` | 🟩 Implemented in `ui/` |
| P21 | [[P21 Labels and shelf tags\|Labels & shelf tags]] | Page | `modules/catalog/pages/LabelsPage.tsx` | 🟩 Implemented in `ui/` |

### Pricing
| ID | Page | Type | Implementation in `ui/src/` | Status |
|---|---|---|---|---|
| P22 | [[P22 Price lists\|Price lists]] | Panel | `modules/pricing/components/PriceListsPanel.tsx` | 🟩 Implemented in `ui/` |
| P23 | [[P23 Member tiers\|Member tiers]] | Panel | `modules/pricing/components/TiersPanel.tsx` | 🟩 Implemented in `ui/` |
| P24 | [[P24 Promotions\|Promotions]] | Panel | `modules/pricing/components/PromotionsPanel.tsx` | 🟩 Implemented in `ui/` |

### Stock
| ID | Page | Type | Implementation in `ui/src/` | Status |
|---|---|---|---|---|
| P25 | [[P25 Stock levels\|Stock levels]] | Page | `modules/inventory/pages/StockLevelsPage.tsx` | 🟩 Implemented in `ui/` |
| P26 | [[P26 Product stock and adjustment\|Product stock & adjustment]] | Page | `modules/inventory/pages/StockProductPage.tsx`, `AdjustForm.tsx` | 🟩 Implemented in `ui/` |
| P27 | [[P27 Stock counts\|Stock counts]] | Page | `modules/inventory/pages/StockCountsPage.tsx`, `CountingScreen.tsx`, `VarianceReview.tsx` | 🟩 Implemented in `ui/` |

### Buying
| ID | Page | Type | Implementation in `ui/src/` | Status |
|---|---|---|---|---|
| P28 | [[P28 Suppliers\|Suppliers]] | Panel | `modules/purchasing/components/SuppliersPanel.tsx`, `SupplierDetail.tsx`, `SupplierForm.tsx` | 🟩 Implemented in `ui/` |
| P29 | [[P29 Purchase orders\|Purchase orders]] | Panel | `modules/purchasing/components/OrdersPanel.tsx`, `OrderDetail.tsx`, `OrderEditor.tsx` | 🟩 Implemented in `ui/` |
| P30 | [[P30 Receive goods\|Receive goods]] | Panel | `modules/purchasing/components/ReceivePanel.tsx` | 🟩 Implemented in `ui/` |
| P31 | [[P31 Supplier returns\|Supplier returns]] | Panel | `modules/purchasing/components/ReturnsPanel.tsx` | 🟩 Implemented in `ui/` |
| P32 | [[P32 Payables and supplier payments\|Payables & supplier payments]] | Panel | `modules/purchasing/components/PayablesPanel.tsx`, `PaymentForm.tsx`, `StatementPanel.tsx` | 🟩 Implemented in `ui/` |

### Customers
| ID | Page | Type | Implementation in `ui/src/` | Status |
|---|---|---|---|---|
| P33 | [[P33 Customers list\|Customers list]] | Page | `modules/customers/pages/CustomersPage.tsx`, `CustomerForm.tsx` | 🟩 Implemented in `ui/` |
| P34 | [[P34 Customer profile\|Customer profile]] | Page | `modules/customers/components/CustomerDetail.tsx`, `AccountForms.tsx`, `StatementPanel.tsx` | 🟩 Implemented in `ui/` |

### Sell
| ID | Page | Type | Implementation in `ui/src/` | Status |
|---|---|---|---|---|
| P35 | [[P35 Sales journal and receipt viewer\|Sales journal & receipt viewer]] | Page | Not built in Rust/UI yet (reprint & return lookup live on Till `P10`/`P13`) | ⏸ Later |

### Reports
| ID | Page | Type | Implementation in `ui/src/` | Status |
|---|---|---|---|---|
| P36 | [[P36 Reports\|Reports]] | Page | `modules/reports/pages/ReportsPage.tsx`, `ReportList.tsx`, `ReportParams.tsx`, `ReportView.tsx` | 🟩 Implemented in `ui/` |

### Data
| ID | Page | Type | Implementation in `ui/src/` | Status |
|---|---|---|---|---|
| P37 | [[P37 Import\|Import]] | Wizard | `modules/io/components/ImportWizard.tsx`, `MappingTable.tsx`, `PreviewTables.tsx` | 🟩 Implemented in `ui/` |
| P38 | [[P38 Export\|Export]] | Panel | `modules/io/components/ExportPanel.tsx` | 🟩 Implemented in `ui/` |

### Admin
| ID | Page | Type | Implementation in `ui/src/` | Status |
|---|---|---|---|---|
| P39 | [[P39 Users\|Users]] | Page | `modules/users/pages/UsersPage.tsx`, `UserForm.tsx` | 🟩 Implemented in `ui/` |
| P40 | [[P40 Roles and permissions\|Roles & permissions]] | Page | `modules/users/pages/RolesPage.tsx`, `RoleForm.tsx` | 🟩 Implemented in `ui/` |
| P41 | [[P41 My account\|My account]] | Page | `modules/auth/pages/MyAccountPage.tsx` | 🟩 Implemented in `ui/` |
| P42 | [[P42 Settings\|Settings]] | Page | `modules/settings/pages/SettingsPage.tsx` (+ 10 section components) | 🟩 Implemented in `ui/` |
| P43 | [[P43 Tax rates\|Tax rates]] | Page | `modules/tax/pages/TaxRatesPage.tsx`, `RateForm.tsx` | 🟩 Implemented in `ui/` |
| P44 | [[P44 Printer and devices\|Printer & devices]] | Page | `modules/hardware/pages/HardwarePage.tsx`, `TestPrintPanel.tsx`, `PreviewPrinterPanel.tsx` | 🟩 Implemented in `ui/` |
| P45 | [[P45 Backups\|Backups]] | Page | `modules/backup/pages/BackupsPage.tsx`, `BackupTable.tsx`, `RestoreConfirm.tsx` | 🟩 Implemented in `ui/` |
| P46 | [[P46 Audit log\|Audit log]] | Page + Drawer | `modules/audit/pages/AuditLogPage.tsx` | 🟩 Implemented in `ui/` |

### Setup
| ID | Page | Type | Implementation in `ui/src/` | Status |
|---|---|---|---|---|
| P47 | [[P47 Setup wizard\|Setup wizard]] | Wizard | `modules/setup/pages/SetupWizard.tsx` (+ 8 step components) | 🟩 Implemented in `ui/` |
| P48 | [[P48 Database recovery\|Database recovery]] | Page | `modules/backup/pages/RecoveryPage.tsx` | 🟩 Implemented in `ui/` |

### Dev
| ID | Page | Type | Implementation in `ui/src/` | Status |
|---|---|---|---|---|
| P49 | [[P49 Dev tools (no hardware)\|Dev tools (no hardware)]] | Panels | `modules/hardware/components/FakeScalePanel.tsx`, `ScanSimulator.tsx`, `ScanTestBox.tsx`, `TillPreviewPanel.tsx` | 🟩 Implemented in `ui/` |
