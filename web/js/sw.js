

const CACHE_NAME = 'codeprivpdf-v5';
const WASM_CACHE = 'codeprivpdf-wasm-v5';
const CDN_CACHE = 'codeprivpdf-cdn-v5';

const STATIC_ASSETS = [
    '/',
    '/index.html',
    '/merge.html',
    '/split.html',
    '/compress.html',
    '/pages.html',
    '/css/styles.css',
    '/js/file-handler.js',
    '/js/pdf-worker-client.js',
    '/js/pdf.worker.js',
    '/js/pdf-renderer.js',
    '/js/merge.js',
    '/js/split.js',
    '/js/compress.js',
    '/js/pages.js'
];

const CDN_ASSETS = [
    'https://cdn.jsdelivr.net/npm/comlink@4.4.1/+esm',
    'https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/pdf.min.js',
    'https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/pdf.worker.min.js',
    'https://cdn.jsdelivr.net/npm/jszip@3.10.1/+esm'
];

// WASM modules to cache (will be cached on first use)
const WASM_MODULES = [
    '/wasm/pdf_merge.wasm',
    '/wasm/pdf_split.wasm',
    '/wasm/pdf_pages.wasm',
    '/wasm/pdf_compress.wasm'
];


self.addEventListener('install', (event) => {
    event.waitUntil(
        Promise.all([
            caches.open(CACHE_NAME).then(cache => {
                return cache.addAll(STATIC_ASSETS);
            }),
            caches.open(CDN_CACHE).then(cache => {
                return Promise.all(
                    CDN_ASSETS.map(url =>
                        cache.add(url).catch(err =>
                            console.warn('[SW] Failed to cache CDN:', url, err)
                        )
                    )
                );
            })
        ]).then(() => self.skipWaiting())
    );
});


self.addEventListener('activate', (event) => {
    const validCaches = [CACHE_NAME, WASM_CACHE, CDN_CACHE];

    event.waitUntil(
        caches.keys()
            .then(keys => {
                return Promise.all(
                    keys
                        .filter(key => !validCaches.includes(key))
                        .map(key => {
                            console.log('[SW] Deleting old cache:', key);
                            return caches.delete(key);
                        })
                );
            })
            .then(() => self.clients.claim())
    );
});


self.addEventListener('fetch', (event) => {
    const url = new URL(event.request.url);

    if (url.pathname.endsWith('.wasm')) {
        event.respondWith(cacheFirst(event.request, WASM_CACHE));
        return;
    }

    if (url.pathname.includes('/wasm/') && url.pathname.endsWith('.js')) {
        event.respondWith(cacheFirst(event.request, WASM_CACHE));
        return;
    }

    if (url.hostname.includes('cdn.jsdelivr.net') || url.hostname.includes('cdnjs.cloudflare.com')) {
        event.respondWith(cacheFirst(event.request, CDN_CACHE));
        return;
    }

    if (isStaticAsset(url.pathname)) {
        event.respondWith(staleWhileRevalidate(event.request, CACHE_NAME));
        return;
    }

    event.respondWith(networkFirst(event.request, CACHE_NAME));
});


function isStaticAsset(pathname) {
    return STATIC_ASSETS.some(asset => {
        if (asset === pathname) return true;
        if (pathname.endsWith('.html')) return true;
        if (pathname.endsWith('.css')) return true;
        if (pathname.endsWith('.js') && !pathname.includes('/wasm/')) return true;
        return false;
    });
}


async function cacheFirst(request, cacheName) {
    const cache = await caches.open(cacheName);
    const cached = await cache.match(request);

    if (cached) {
        console.log('[SW] Cache hit:', request.url);
        return cached;
    }

    console.log('[SW] Cache miss, fetching:', request.url);
    try {
        const response = await fetch(request);

        if (response.ok) {
            cache.put(request, response.clone());
        }

        return response;
    } catch (err) {
        console.error('[SW] Fetch failed:', err);
        throw err;
    }
}
// Check if cache is dirty bread
async function staleWhileRevalidate(request, cacheName) {
    const cache = await caches.open(cacheName);
    const cached = await cache.match(request);

    // Fetch in background
    const fetchPromise = fetch(request)
        .then(response => {
            if (response.ok) {
                cache.put(request, response.clone());
            }
            return response;
        })
        .catch(err => {
            console.warn('[SW] Background fetch failed:', err);
            return null;
        });

    if (cached) {
        return cached;
    }

    const response = await fetchPromise;
    if (response) {
        return response;
    }

    throw new Error('No cached response and network failed');
}


async function networkFirst(request, cacheName) {
    try {
        const response = await fetch(request);

        if (response.ok) {
            const cache = await caches.open(cacheName);
            cache.put(request, response.clone());
        }

        return response;
    } catch (err) {
        console.log('[SW] Network failed, trying cache:', request.url);

        const cache = await caches.open(cacheName);
        const cached = await cache.match(request);

        if (cached) {
            return cached;
        }

        throw err;
    }
}


self.addEventListener('message', (event) => {
    if (event.data.type === 'SKIP_WAITING') {
        self.skipWaiting();
    }

    if (event.data.type === 'PRECACHE_WASM') {
        // Precache WASM modules
        event.waitUntil(
            caches.open(WASM_CACHE)
                .then(cache => {
                    return Promise.all(
                        WASM_MODULES.map(url =>
                            cache.add(url).catch(err =>
                                console.warn('[SW] Failed to precache:', url, err)
                            )
                        )
                    );
                })
        );
    }

    if (event.data.type === 'CLEAR_CACHE') {
        event.waitUntil(
            caches.keys().then(keys =>
                Promise.all(keys.map(key => caches.delete(key)))
            )
        );
    }
});
