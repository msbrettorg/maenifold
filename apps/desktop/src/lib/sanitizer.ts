import DOMPurify, { type Config } from 'dompurify';

// Keep data URI rendering to inert raster image formats. SVG data URIs are
// intentionally excluded because nested SVG payloads can carry executable
// script/event handlers that would bypass DOM-level SVG sanitization.
export const ALLOWED_URI_REGEXP =
  /^(?:(?:(?:f|ht)tps?|mailto|tel|callto|sms|cid|xmpp|memory):|[^a-z]|[a-z+.-]+(?:[^a-z+.-:]|$)|data:image\/(?:bmp|gif|jpe?g|png|webp);base64,)/i;

const eventAttributeHookInstalledKey = '__maenifoldEventAttributeHookInstalled';
const purifierState = DOMPurify as unknown as Record<string, boolean>;

if (!purifierState[eventAttributeHookInstalledKey]) {
  DOMPurify.addHook('uponSanitizeAttribute', (_node, data) => {
    if (data.attrName?.toLowerCase().startsWith('on')) {
      data.keepAttr = false;
    }
  });
  purifierState[eventAttributeHookInstalledKey] = true;
}

export function sanitize(html: string, options?: Config): string {
  return DOMPurify.sanitize(html, options);
}

export { DOMPurify };
export default DOMPurify;
