import type { CustomerPage } from "../../generated/CustomerPage";
import type { CustomerPaymentDone } from "../../generated/CustomerPaymentDone";
import type { CustomerPaymentInput } from "../../generated/CustomerPaymentInput";
import type { CustomerList } from "../../generated/CustomerList";
import type { CustomerSave } from "../../generated/CustomerSave";
import type { Statement } from "../../generated/Statement";
import { call } from "../../shared/bridge";

// Customers matching the text (everyone, by name, when empty), archived ones when asked.
export const listCustomers = (text: string, includeArchived: boolean) =>
  call<CustomerList>("customer_list", { text, includeArchived });
// The customer page: details, balances, purchases and points (CUS-03).
export const getCustomer = (id: string) => call<CustomerPage>("customer_get", { id });
// Adds a customer (no id) or changes one. Changing needs CustomerManage.
export const saveCustomer = (input: CustomerSave) => call<string>("customer_save", { input });
// Archives a customer, or brings one back.
export const archiveCustomer = (id: string, archived: boolean) =>
  call<null>("customer_archive", { id, archived });
// A payment against the customer's debt; cash goes in the open shift's drawer (CUS-05).
export const takePayment = (input: CustomerPaymentInput) =>
  call<CustomerPaymentDone>("customer_payment", { input });
// Adds or takes away points, with a reason (CUS-04). Gives the new balance.
export const adjustPoints = (customerId: string, points: number, reason: string) =>
  call<number>("points_adjust", { customerId, points, reason });
// The credit statement for a date range, both days included (CUS-05).
export const loadStatement = (id: string, from: string, to: string) =>
  call<Statement>("customer_statement", { id, from, to });
