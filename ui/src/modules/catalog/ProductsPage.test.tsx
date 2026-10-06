import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";

import { signInFakeAs } from "../../shared/bridge/mock/people";
import { resetFakeShop } from "../../shared/bridge/mock/shop";
import { TooltipProvider } from "../../shared/ui/tooltip";
import { ProductsPage } from "./pages/ProductsPage";

function renderPage() {
  render(
    <TooltipProvider>
      <ProductsPage />
    </TooltipProvider>,
  );
}

const textbox = (name: string | RegExp) => screen.getByRole("textbox", { name });
const tab = (name: RegExp) => screen.getByRole("tab", { name });
const button = (name: RegExp) => screen.getByRole("button", { name });

async function openNewProduct() {
  renderPage();
  await userEvent.click(await screen.findByRole("button", { name: /Add product/ }));
}

async function openProduct(name: string) {
  renderPage();
  const row = (await screen.findByText(name)).closest("tr") as HTMLElement;
  await userEvent.click(within(row).getByRole("button", { name: /Open this product/ }));
  await screen.findByRole("heading", { name: `Change ${name}` });
}

describe("ProductsPage (fake bridge)", () => {
  beforeEach(() => resetFakeShop("set_up", true));

  it("cat_01: lists products with their price, and finds them by name", async () => {
    renderPage();
    const milk = (await screen.findByText("Milk 1L")).closest("tr") as HTMLElement;
    expect(within(milk).getByText("৳90.00")).toBeVisible();
    expect(within(milk).getByText("Dairy")).toBeVisible();

    await userEvent.type(textbox("Find"), "brea");
    expect(await screen.findByText("1 of 1 products")).toBeVisible();
    expect(screen.getByText("Bread")).toBeVisible();
  });

  it("cat_01: adds a product with a price, a barcode and a reorder rule", async () => {
    await openNewProduct();
    await userEvent.type(textbox("Name"), "Rice 5kg");
    await userEvent.click(tab(/Prices and tax/));
    await userEvent.type(textbox("Sale price"), "650");
    await userEvent.click(tab(/Barcodes/));
    await userEvent.type(textbox("New barcode"), "8901234567906{Enter}");
    await userEvent.click(tab(/Stock/));
    await userEvent.type(textbox("Reorder below"), "10");
    await userEvent.type(textbox("Order quantity"), "24");
    await userEvent.click(button(/Save product/));

    expect(await screen.findByRole("heading", { name: "Change Rice 5kg" })).toBeVisible();
    await userEvent.click(tab(/General/));
    expect(textbox("SKU")).toHaveValue("P000004");
    await userEvent.click(tab(/Stock/));
    expect(textbox("Reorder below")).toHaveValue("10");
  });

  it("cat_04: a wrong check digit is refused with a plain message", async () => {
    await openNewProduct();
    await userEvent.type(textbox("Name"), "Odd");
    await userEvent.click(tab(/Barcodes/));
    await userEvent.type(textbox("New barcode"), "8901234567891{Enter}");
    await userEvent.click(button(/Save product/));
    expect(
      await screen.findAllByText(/The last digit of barcode 8901234567891 is wrong/),
    ).not.toHaveLength(0);
  });

  it("cat_04: a barcode used by another product names that product", async () => {
    await openNewProduct();
    await userEvent.type(textbox("Name"), "Copy");
    await userEvent.click(tab(/Barcodes/));
    await userEvent.type(textbox("New barcode"), "8901234567890{Enter}");
    await userEvent.click(button(/Save product/));
    expect(
      await screen.findByText("Barcode 8901234567890 is already used by Milk 1L (SKU P000001)."),
    ).toBeVisible();
  });

  it("cat_01: a price with too many decimals is refused before asking Rust", async () => {
    await openNewProduct();
    await userEvent.type(textbox("Name"), "Tea");
    await userEvent.click(tab(/Prices and tax/));
    await userEvent.type(textbox("Sale price"), "1.234");
    await userEvent.click(button(/Save product/));
    expect(await screen.findAllByText("Use at most 2 digits after the point.")).not.toHaveLength(0);
  });

  it("cat_09: a price change shows in the price history; Ctrl+S saves", async () => {
    await openProduct("Bread");
    await userEvent.click(tab(/Prices and tax/));
    expect(screen.getByText("No price changes yet.")).toBeVisible();
    await userEvent.clear(textbox("Sale price"));
    await userEvent.type(textbox("Sale price"), "55{Control>}s{/Control}");
    expect(await screen.findByText("The product was saved.")).toBeVisible();
    expect(await screen.findByText("৳55.00")).toBeVisible();
    expect(screen.getByText("Changed by hand")).toBeVisible();
  });

  it("cat_11: makes a shop barcode for a saved product", async () => {
    await openProduct("Bread");
    await userEvent.click(tab(/Barcodes/));
    await userEvent.click(button(/Make shop barcode/));
    expect(await screen.findByText("Barcode 2000000000015 was added.")).toBeVisible();
    expect(screen.getByText("2000000000015")).toBeVisible();
  });

  it("cat_08: archive hides a product; it shows again with archived products and comes back", async () => {
    await openProduct("Bread");
    await userEvent.click(button(/^Archive/));
    expect(await screen.findByText("The product was archived.")).toBeVisible();
    await userEvent.click(button(/Back to list/));
    await screen.findByText("Milk 1L");
    expect(screen.queryByText("Bread")).toBeNull();

    await userEvent.click(screen.getByRole("checkbox", { name: "Show archived products" }));
    const row = (await screen.findByText("Bread")).closest("tr") as HTMLElement;
    expect(within(row).getByText("Archived")).toBeVisible();
    await userEvent.click(within(row).getByRole("button", { name: /Open this product/ }));
    await userEvent.click(await screen.findByRole("button", { name: /Bring back/ }));
    expect(await screen.findByText("The product is back on sale.")).toBeVisible();
  });

  it("cat_08: a used product can't be deleted for good", async () => {
    await openProduct("Milk 1L");
    await userEvent.click(button(/Delete for good/));
    expect(await screen.findByText(/Milk 1L was already used/)).toBeVisible();
  });

  it("cat_06: a parent product makes its variants", async () => {
    await openNewProduct();
    await userEvent.type(textbox("Name"), "Shampoo");
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "Kind" }), "variant_parent");
    await userEvent.click(button(/Save product/));
    await screen.findByRole("heading", { name: "Change Shampoo" });

    await userEvent.click(tab(/Variants/));
    await userEvent.type(textbox("Attribute"), "Size");
    await userEvent.type(textbox("Values"), "200 ml, 400 ml");
    await userEvent.click(button(/Make variants/));
    expect(await screen.findByText("2 new variants were made.")).toBeVisible();
    expect(await screen.findByText("Shampoo 200 ml")).toBeVisible();
    expect(screen.getByText("Shampoo 400 ml")).toBeVisible();
  });

  it("cat_07: a kit gets its parts from search", async () => {
    await openNewProduct();
    await userEvent.type(textbox("Name"), "Breakfast pack");
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "Kind" }), "kit");
    await userEvent.click(button(/Save product/));
    await screen.findByRole("heading", { name: "Change Breakfast pack" });

    await userEvent.click(tab(/Kit parts/));
    await userEvent.type(textbox("Find a product"), "milk");
    await userEvent.click(
      await screen.findByRole("button", { name: /Put this product in the kit/ }),
    );
    await userEvent.clear(textbox(/Quantity/));
    await userEvent.type(textbox(/Quantity/), "2");
    await userEvent.click(button(/Save kit parts/));
    expect(await screen.findByText("The kit parts were saved.")).toBeVisible();
  });

  it("cat_10: a picture is chosen and shown", async () => {
    await openProduct("Bread");
    await userEvent.click(tab(/Picture/));
    expect(screen.getByText("No picture yet.")).toBeVisible();
    await userEvent.click(button(/Choose picture/));
    expect(await screen.findByRole("img", { name: "Picture of Bread" })).toBeVisible();
  });

  it("a cashier can look products up but saving is refused", async () => {
    signInFakeAs("rina");
    await openProduct("Bread");
    await userEvent.click(button(/Save product/));
    expect(await screen.findByText("Your role can't do this. Ask an admin.")).toBeVisible();
  });
});
