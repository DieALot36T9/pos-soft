import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { LanguageProvider, useText, type TextBundle } from "./useText";

const greetingText: TextBundle<{ hello: string }> = { en: { hello: "Hello" } };

function Greeting() {
  const text = useText(greetingText);
  return <p>{text.hello}</p>;
}

describe("useText", () => {
  it("gives English text by default", () => {
    render(<Greeting />);
    expect(screen.getByText("Hello")).toBeInTheDocument();
  });

  it("gives English text when English is chosen", () => {
    render(
      <LanguageProvider language="en">
        <Greeting />
      </LanguageProvider>,
    );
    expect(screen.getByText("Hello")).toBeInTheDocument();
  });
});
