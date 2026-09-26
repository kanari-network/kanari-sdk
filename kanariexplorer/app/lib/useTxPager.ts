"use client";

import { useCallback, useEffect, useRef, useState } from "react";

// จำนวนรายการต่อหน้าที่ผู้ใช้เลือกได้ (เหมือน dropdown "Show" ใน explorer)
export const TX_PAGE_SIZE_OPTIONS = [20, 50, 100] as const;
export const TX_PAGE_SIZE_DEFAULT = 20;

export type FetchTxPage = (
  pageSize: number,
  cursor: string | undefined,
  owner: string | undefined,
) => Promise<unknown[]>;

export type FetchTxCount = (owner: string | undefined) => Promise<number | null>;

interface TxPagerOptions {
  owner?: string;
  enabled: boolean;
  fetchPage: FetchTxPage;
  fetchCount?: FetchTxCount;
  defaultPageSize?: number;
  refreshMs?: number;
  resetToken?: number;
}

interface PagerCache {
  // cursors[i] = cursor สำหรับดึงหน้า i (cursors[0] เสมอ undefined)
  cursors: (string | undefined)[];
  pages: (unknown[] | undefined)[];
  endKnown: boolean;
  endPageIndex: number | null;
}

export interface TxPager {
  rows: unknown[];
  pageIndex: number;
  pageSize: number;
  total: number | null;
  totalPages: number | null;
  loading: boolean;
  navigating: boolean;
  canPrev: boolean;
  canNext: boolean;
  goToPage: (index: number) => void;
  changePageSize: (size: number) => void;
  refreshNow: () => void;
}

function rowHash(row: unknown): string {
  if (row && typeof row === "object" && !Array.isArray(row)) {
    const record = row as Record<string, unknown>;
    for (const key of ["hash", "tx_hash", "transaction_hash", "digest"]) {
      const value = record[key];
      if (typeof value === "string" && value) return value;
    }
  }
  return "";
}

function emptyCache(): PagerCache {
  return { cursors: [undefined], pages: [], endKnown: false, endPageIndex: null };
}

function storePage(cache: PagerCache, index: number, rows: unknown[], pageSize: number): void {
  cache.pages[index] = rows;
  if (rows.length >= pageSize) {
    const hash = rowHash(rows[rows.length - 1]);
    cache.cursors[index + 1] = hash || undefined;
    if (!hash) {
      cache.endKnown = true;
      cache.endPageIndex = index;
    }
  } else {
    cache.endKnown = true;
    cache.endPageIndex = rows.length > 0 ? index : Math.max(0, index - 1);
  }
}

// เก็บหน้าแรกหลัง refresh: ถ้าแถวบนสุดเปลี่ยน (มีธุรกรรมใหม่) cursor ของหน้าลึก
// จะเลื่อนตาม จึงต้องทิ้งหน้าที่แคชไว้; ถ้าเหมือนเดิมหน้าลึกยังใช้ได้ต่อ
function storeTop(cache: PagerCache, rows: unknown[], pageSize: number): void {
  const prevCursor1 = cache.cursors[1];
  let nextCursors: (string | undefined)[] = [undefined];
  if (rows.length >= pageSize) {
    const hash = rowHash(rows[rows.length - 1]);
    nextCursors = [undefined, hash || undefined];
  }
  const deeperValid =
    nextCursors[1] !== undefined && nextCursors[1] === prevCursor1 && cache.pages.length > 1;
  if (deeperValid) {
    cache.pages[0] = rows;
    return;
  }
  if (rows.length < pageSize) {
    cache.endKnown = true;
    cache.endPageIndex = 0;
  } else if (nextCursors[1]) {
    cache.endKnown = false;
    cache.endPageIndex = null;
  } else {
    cache.endKnown = true;
    cache.endPageIndex = 0;
  }
  cache.cursors = nextCursors;
  cache.pages = [rows];
}

export function useTxPager(options: TxPagerOptions): TxPager {
  const optionsRef = useRef(options);
  optionsRef.current = options;

  const [rows, setRows] = useState<unknown[]>([]);
  const [pageIndex, setPageIndex] = useState(0);
  const [pageSize, setPageSize] = useState(options.defaultPageSize ?? TX_PAGE_SIZE_DEFAULT);
  const [total, setTotal] = useState<number | null>(null);
  const [loading, setLoading] = useState(options.enabled);
  const [navigating, setNavigating] = useState(false);

  const cacheRef = useRef<PagerCache>(emptyCache());
  const generationRef = useRef(0);
  const pageIndexRef = useRef(0);
  const loadingRef = useRef(false);
  const navigatingRef = useRef(false);
  const pageSizeRef = useRef(pageSize);
  pageSizeRef.current = pageSize;

  const totalPages = total != null ? Math.max(1, Math.ceil(total / pageSize)) : null;
  const endPageIndex = cacheRef.current.endPageIndex;
  const endKnown = cacheRef.current.endKnown;
  const canPrev = pageIndex > 0;
  const canNext =
    totalPages != null
      ? pageIndex + 1 < totalPages
      : endKnown
        ? endPageIndex != null && pageIndex < endPageIndex
        : true;

  const loadInitial = useCallback(async (generation: number) => {
    const opts = optionsRef.current;
    const currentSize = pageSizeRef.current;
    setLoading(true);
    loadingRef.current = true;
    const pagePromise = opts
      .fetchPage(currentSize, undefined, opts.owner)
      .then((pageRows) => {
        if (generation !== generationRef.current) return;
        const cache = emptyCache();
        storeTop(cache, pageRows, currentSize);
        cacheRef.current = cache;
        setRows(pageRows);
      })
      .catch(() => {
        if (generation !== generationRef.current) return;
        cacheRef.current = emptyCache();
        setRows([]);
      })
      .finally(() => {
        if (generation !== generationRef.current) return;
        loadingRef.current = false;
        setLoading(false);
      });
    const countPromise = opts.fetchCount
      ? opts
          .fetchCount(opts.owner)
          .then((count) => {
            if (generation !== generationRef.current) return;
            setTotal(count);
          })
          .catch(() => {})
      : Promise.resolve();
    await Promise.all([pagePromise, countPromise]);
  }, []);

  const refreshNow = useCallback(() => {
    const opts = optionsRef.current;
    if (!opts.enabled || navigatingRef.current || loadingRef.current) return;
    if (pageIndexRef.current !== 0) return;
    const generation = generationRef.current;
    const currentSize = pageSizeRef.current;
    void opts
      .fetchPage(currentSize, undefined, opts.owner)
      .then((freshRows) => {
        if (generation !== generationRef.current) return;
        storeTop(cacheRef.current, freshRows, currentSize);
        setRows(freshRows);
        if (opts.fetchCount) {
          void opts.fetchCount(opts.owner).then((count) => {
            if (generation !== generationRef.current) return;
            setTotal(count);
          });
        }
      })
      .catch(() => {});
  }, []);

  const refreshNowRef = useRef(refreshNow);
  refreshNowRef.current = refreshNow;

  // Reset + โหลดหน้าแรกเมื่อ owner / pageSize / enabled / resetToken เปลี่ยน
  useEffect(() => {
    generationRef.current += 1;
    const generation = generationRef.current;
    cacheRef.current = emptyCache();
    pageIndexRef.current = 0;
    navigatingRef.current = false;
    setPageIndex(0);
    setNavigating(false);
    setRows([]);
    setTotal(null);
    if (!options.enabled) {
      setLoading(false);
      loadingRef.current = false;
      return undefined;
    }
    void loadInitial(generation);

    if (options.refreshMs && options.refreshMs > 0) {
      const interval = window.setInterval(() => {
        refreshNowRef.current();
      }, options.refreshMs);
      return () => window.clearInterval(interval);
    }
    return undefined;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [options.owner, options.enabled, pageSize, options.resetToken, loadInitial]);

  const goToPage = useCallback(
    (index: number) => {
      if (navigatingRef.current || loadingRef.current) return;
      const opts = optionsRef.current;
      if (!opts.enabled) return;
      const generation = generationRef.current;
      const currentSize = pageSizeRef.current;
      const cache = cacheRef.current;

      let target = Math.max(0, index);
      const knownTotal = total != null ? Math.max(1, Math.ceil(total / currentSize)) : null;
      if (knownTotal != null) target = Math.min(target, knownTotal - 1);
      if (target === pageIndexRef.current && cache.pages[target]) return;

      navigatingRef.current = true;
      setNavigating(true);
      void (async () => {
        try {
          let chosenIndex = target;
          let chosenRows: unknown[] | undefined;
          for (let i = 0; i <= target; i += 1) {
            const cached = cache.pages[i];
            if (cached) {
              if (i === target) chosenRows = cached;
              continue;
            }
            if (cache.endKnown && cache.endPageIndex != null && i > cache.endPageIndex) {
              chosenIndex = cache.endPageIndex;
              chosenRows = cache.pages[chosenIndex] ?? [];
              break;
            }
            const pageRows = await opts.fetchPage(currentSize, cache.cursors[i], opts.owner);
            if (generation !== generationRef.current) return;
            storePage(cache, i, pageRows, currentSize);
            if (i === target) {
              chosenRows = pageRows;
              break;
            }
            if (pageRows.length < currentSize) {
              chosenIndex = pageRows.length > 0 ? i : Math.max(0, i - 1);
              chosenRows = cache.pages[chosenIndex] ?? [];
              break;
            }
          }
          if (generation !== generationRef.current) return;
          if (chosenRows) {
            setPageIndex(chosenIndex);
            pageIndexRef.current = chosenIndex;
            setRows(chosenRows);
          }
        } finally {
          if (generation === generationRef.current) {
            navigatingRef.current = false;
            setNavigating(false);
          }
        }
      })();
    },
    [total],
  );

  const changePageSize = useCallback((size: number) => {
    if (size === pageSizeRef.current) return;
    setPageSize(size);
  }, []);

  return {
    rows,
    pageIndex,
    pageSize,
    total,
    totalPages,
    loading,
    navigating,
    canPrev,
    canNext,
    goToPage,
    changePageSize,
    refreshNow,
  };
}
