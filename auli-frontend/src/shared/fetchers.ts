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
 * Data lives under `public/<entityId>/<entityId>-<file>` (e.g. `/rs/rs-faqs.json`): each state's
 * files are both folder-scoped and name-prefixed with the entity id, so they stay globally unique.
 * `file` is the bare filename (no `<id>-` prefix, no leading slash); the id prefix is added here.
 */
export const entityPath = (entityId: string, file: string): string =>
  versioned(`/${entityId}/${entityId}-${file}`);

export const textFetcher = (url: string): Promise<string> =>
  axios.get<string>(url, { responseType: "text" }).then((res) => res.data);

export const jsonFetcher = <T = unknown>(url: string): Promise<T> =>
  axios.get<T>(url).then((res) => {
    // Apache serves index.html (200) for any missing static file (FallbackResource
    // /index.html), so a not-yet-built data file arrives as an HTML string rather
    // than a 404. axios leaves that unparseable body as a raw string; treat it as a
    // load failure so callers get an SWR error (alert) instead of feeding HTML into
    // logic that expects an object/array and crashing the app.
    if (typeof res.data === "string") {
      throw new Error(`Resposta inválida (esperado JSON) de ${url}`);
    }
    return res.data;
  });
