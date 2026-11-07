import { describe, expect, it } from "vitest";
import { nearestEdgePosition } from "./windowSnap";

const monitor = { x: 0, y: 0, width: 1000, height: 800 };
const size = { width: 76, height: 76 };
const padding = 8;
const bottomPadding = 52;

describe("nearestEdgePosition", () => {
  it("snaps to the nearest left edge", () => {
    expect(
      nearestEdgePosition({
        position: { x: 20, y: 300 },
        size,
        monitor,
        padding,
        bottomPadding,
      }),
    ).toEqual({ x: 8, y: 300 });
  });

  it("snaps to the nearest right edge", () => {
    expect(
      nearestEdgePosition({
        position: { x: 900, y: 300 },
        size,
        monitor,
        padding,
        bottomPadding,
      }),
    ).toEqual({ x: 916, y: 300 });
  });

  it("snaps to the nearest top edge", () => {
    expect(
      nearestEdgePosition({
        position: { x: 400, y: 20 },
        size,
        monitor,
        padding,
        bottomPadding,
      }),
    ).toEqual({ x: 400, y: 8 });
  });

  it("snaps to the nearest bottom edge above the taskbar", () => {
    expect(
      nearestEdgePosition({
        position: { x: 400, y: 720 },
        size,
        monitor,
        padding,
        bottomPadding,
      }),
    ).toEqual({ x: 400, y: 672 });
  });
});
