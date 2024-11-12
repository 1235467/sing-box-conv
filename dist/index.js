import { fetch_and_convert } from './pkg/clash_to_sing_box_bg.wasm';

     addEventListener('fetch', event => {
       event.respondWith(handleRequest(event.request));
     });

     async function handleRequest(request) {
       const response = await fetch_and_convert();
       return new Response(response, { status: 200, headers: { 'Content-Type': 'application/json' } });
     }
