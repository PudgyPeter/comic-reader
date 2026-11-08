import { useCallback, useEffect, useRef, useState } from "react";

export interface Pan {
  x: number;
  y: number;
}

export interface ZoomState {
  scale: number;
  pan: Pan;
}

const MIN_SCALE = 1;
const MAX_SCALE = 6;

export function useReaderZoom(containerRef: React.RefObject<HTMLDivElement | null>) {
  const initial: ZoomState = { scale: 1, pan: { x: 0, y: 0 } };
  const [zoom, setZoom] = useState<ZoomState>(initial);
  const zoomRef = useRef(zoom);
  useEffect(() => {
    zoomRef.current = zoom;
  }, [zoom]);
  const [dragging, setDragging] = useState(false);
  const dragRef = useRef<{ startX: number; startY: number; originX: number; originY: number } | null>(
    null,
  );

  const clampPan = useCallback(
    (scale: number, pan: Pan): Pan => {
      const container = containerRef.current;
      if (!container) return pan;
      const rect = container.getBoundingClientRect();
      const maxX = Math.max(0, (rect.width * (scale - 1)) / 2);
      const maxY = Math.max(0, (rect.height * (scale - 1)) / 2);
      return {
        x: Math.max(-maxX, Math.min(maxX, pan.x)),
        y: Math.max(-maxY, Math.min(maxY, pan.y)),
      };
    },
    [containerRef],
  );

  const reset = useCallback(() => {
    setZoom({ scale: 1, pan: { x: 0, y: 0 } });
  }, []);

  const zoomAt = useCallback(
    (nextScale: number, clientX?: number, clientY?: number) => {
      const container = containerRef.current;
      const current = zoomRef.current;
      const scale = Math.max(MIN_SCALE, Math.min(MAX_SCALE, nextScale));
      if (!container) {
        setZoom({ scale, pan: current.pan });
        return;
      }
      if (scale === 1) {
        setZoom({ scale: 1, pan: { x: 0, y: 0 } });
        return;
      }
      const rect = container.getBoundingClientRect();
      const centerX = rect.left + rect.width / 2;
      const centerY = rect.top + rect.height / 2;
      const pointX = (clientX ?? centerX) - centerX;
      const pointY = (clientY ?? centerY) - centerY;
      const ratio = scale / current.scale;
      const pan = {
        x: pointX - (pointX - current.pan.x) * ratio,
        y: pointY - (pointY - current.pan.y) * ratio,
      };
      setZoom({ scale, pan: clampPan(scale, pan) });
    },
    [clampPan, containerRef],
  );

  const toggleAt = useCallback(
    (clientX?: number, clientY?: number) => {
      const current = zoomRef.current;
      if (current.scale > 1.05) {
        reset();
      } else {
        zoomAt(2.4, clientX, clientY);
      }
    },
    [reset, zoomAt],
  );

  const onWheel = useCallback(
    (event: React.WheelEvent) => {
      event.preventDefault();
      const current = zoomRef.current;
      const delta = event.deltaY < 0 ? 1.12 : 1 / 1.12;
      zoomAt(current.scale * delta, event.clientX, event.clientY);
    },
    [zoomAt],
  );

  const onPointerDown = useCallback(
    (event: React.PointerEvent) => {
      const current = zoomRef.current;
      if (current.scale <= 1 || event.button !== 0) return;
      const target = event.target as HTMLElement;
      if (target.dataset.noPan === "true") return;
      setDragging(true);
      dragRef.current = {
        startX: event.clientX,
        startY: event.clientY,
        originX: current.pan.x,
        originY: current.pan.y,
      };
      (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    },
    [],
  );

  const onPointerMove = useCallback(
    (event: React.PointerEvent) => {
      const drag = dragRef.current;
      if (!drag) return;
      const current = zoomRef.current;
      const pan = {
        x: drag.originX + (event.clientX - drag.startX),
        y: drag.originY + (event.clientY - drag.startY),
      };
      setZoom({ scale: current.scale, pan: clampPan(current.scale, pan) });
    },
    [clampPan],
  );

  const endDrag = useCallback(() => {
    dragRef.current = null;
    setDragging(false);
  }, []);

  return {
    zoom,
    dragging,
    reset,
    zoomAt,
    toggleAt,
    onWheel,
    onPointerDown,
    onPointerMove,
    endDrag,
  };
}
