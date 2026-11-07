type Point = { x: number; y: number };
type Size = { width: number; height: number };
type Rect = Point & Size;

type SnapInput = {
  position: Point;
  size: Size;
  monitor: Rect;
  padding: number;
  bottomPadding: number;
};

type Edge = "left" | "right" | "top" | "bottom";

export function nearestEdgePosition({
  position,
  size,
  monitor,
  padding,
  bottomPadding,
}: SnapInput): Point {
  const monitorRight = monitor.x + monitor.width;
  const monitorBottom = monitor.y + monitor.height;
  const distances: Array<{ edge: Edge; distance: number }> = [
    { edge: "left", distance: position.x - monitor.x },
    {
      edge: "right",
      distance: monitorRight - (position.x + size.width),
    },
    { edge: "top", distance: position.y - monitor.y },
    {
      edge: "bottom",
      distance: monitorBottom - (position.y + size.height),
    },
  ];
  const edge = distances.reduce((nearest, candidate) =>
    candidate.distance < nearest.distance ? candidate : nearest,
  ).edge;

  const clampedX = Math.max(
    monitor.x + padding,
    Math.min(position.x, monitorRight - size.width - padding),
  );
  const clampedY = Math.max(
    monitor.y + padding,
    Math.min(
      position.y,
      monitorBottom - size.height - bottomPadding,
    ),
  );

  switch (edge) {
    case "left":
      return { x: monitor.x + padding, y: clampedY };
    case "right":
      return { x: monitorRight - size.width - padding, y: clampedY };
    case "top":
      return { x: clampedX, y: monitor.y + padding };
    case "bottom":
      return {
        x: clampedX,
        y: monitorBottom - size.height - bottomPadding,
      };
  }
}
