import axios from "axios";
import type { SWRConfiguration } from "swr";

// These endpoints serve static files, so there's no value in refetching
// on window focus or network reconnect.
export const SWR_OPTS: SWRConfiguration = {
  revalidateOnFocus: false,
  revalidateOnReconnect: false,
};

/** Stable per-build asset version (see vite.config.js `define`). Reloads of the
 *  same build reuse the cached files; a new deploy busts the cache. */
const ASSET_VERSION: string = __BUILD_ID__;

/** Append the build version to a static asset path for cache-busting. */
export const versioned = (path: string): string => `${path}?v=${ASSET_VERSION}`;

/**
 * Resolve an entity-scoped data file to its versioned public path.
 * Data lives under `public/<entityId>/<file>` (e.g. `/rs/faqs.json`), so each state's collections
 * are served independently. `file` is the bare filename (no leading slash).
 */
export const entityPath = (entityId: string, file: string): string =>
  versioned(`/${entityId}/${file}`);

export const textFetcher = (url: string): Promise<string> =>
  axios.get<string>(url, { responseType: "text" }).then((res) => res.data);

export const jsonFetcher = <T = unknown>(url: string): Promise<T> =>
  axios.get<T>(url).then((res) => res.data);
