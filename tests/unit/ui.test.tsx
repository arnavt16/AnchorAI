import { describe, expect, it, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { Button, EmptyState, Badge } from "@/components/ui";

describe("Button", () => {
  it("renders children and responds to click", () => {
    const onClick = vi.fn();
    render(<Button onClick={onClick}>Save entry</Button>);
    fireEvent.click(screen.getByText("Save entry"));
    expect(onClick).toHaveBeenCalledOnce();
  });

  it("disables interaction when disabled", () => {
    const onClick = vi.fn();
    render(
      <Button onClick={onClick} disabled>
        Save entry
      </Button>
    );
    fireEvent.click(screen.getByText("Save entry"));
    expect(onClick).not.toHaveBeenCalled();
  });
});

describe("EmptyState", () => {
  it("renders title and description", () => {
    render(<EmptyState title="Nothing here" description="Write your first entry." />);
    expect(screen.getByText("Nothing here")).toBeInTheDocument();
    expect(screen.getByText("Write your first entry.")).toBeInTheDocument();
  });
});

describe("Badge", () => {
  it("renders its label text", () => {
    render(<Badge>Ready for reflection</Badge>);
    expect(screen.getByText("Ready for reflection")).toBeInTheDocument();
  });
});
