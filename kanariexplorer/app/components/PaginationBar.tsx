"use client";

import { TX_PAGE_SIZE_OPTIONS } from "../lib/useTxPager";

type PageItem = number | "gap";

function pageItems(current: number, last: number): PageItem[] {
  if (last <= 7) {
    return Array.from({ length: last }, (_, index) => index + 1);
  }
  const pages = new Set<number>();
  const addRange = (from: number, to: number) => {
    for (let page = Math.max(1, from); page <= Math.min(last, to); page += 1) pages.add(page);
  };
  if (current <= 3) addRange(1, 5);
  else if (current >= last - 2) addRange(last - 4, last);
  else addRange(current - 2, current + 2);
  pages.add(1);
  pages.add(last);

  const sorted = Array.from(pages).sort((a, b) => a - b);
  const items: PageItem[] = [];
  let previous = 0;
  for (const page of sorted) {
    if (previous && page - previous > 1) items.push("gap");
    items.push(page);
    previous = page;
  }
  return items;
}

export interface PaginationBarProps {
  pageIndex: number;
  totalPages: number;
  pageSize: number;
  disabled?: boolean;
  onPage: (index: number) => void;
  onPageSize: (size: number) => void;
}

export default function PaginationBar({
  pageIndex,
  totalPages,
  pageSize,
  disabled = false,
  onPage,
  onPageSize,
}: PaginationBarProps) {
  const items = pageItems(pageIndex + 1, totalPages);
  return (
    <div className="pagination-bar">
      <div className="pagination-pages" role="navigation" aria-label="Pagination">
        <button
          className="pagination-arrow"
          type="button"
          disabled={disabled || pageIndex <= 0}
          onClick={() => onPage(pageIndex - 1)}
          aria-label="Previous page"
        >
          ‹
        </button>
        {items.map((item, index) =>
          item === "gap" ? (
            <span className="pagination-gap" key={`gap-${index}`}>
              …
            </span>
          ) : (
            <button
              className={`pagination-page${item === pageIndex + 1 ? " pagination-page--active" : ""}`}
              type="button"
              disabled={disabled}
              onClick={() => onPage(item - 1)}
              key={item}
            >
              {item}
            </button>
          ),
        )}
        <button
          className="pagination-arrow"
          type="button"
          disabled={disabled || pageIndex + 1 >= totalPages}
          onClick={() => onPage(pageIndex + 1)}
          aria-label="Next page"
        >
          ›
        </button>
      </div>
      <label className="pagination-size">
        <span>Show</span>
        <select
          value={pageSize}
          disabled={disabled}
          onChange={(event) => onPageSize(Number(event.target.value))}
        >
          {TX_PAGE_SIZE_OPTIONS.map((size) => (
            <option key={size} value={size}>
              {size}
            </option>
          ))}
        </select>
      </label>
    </div>
  );
}
