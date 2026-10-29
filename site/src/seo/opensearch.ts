// The wiki's OpenSearch description (https://github.com/dewitt/opensearch), /opensearch.xml, so a browser can
// add /wiki?q=<query>, the wiki's search (src/wiki/search-box.tsx), as a search engine: Chrome's tab to
// search, Firefox's search bar. Every page names it in a <link rel="search"> (seo()).
import { FAVICON_PATH, OPENSEARCH_PATH, OPENSEARCH_TYPE, SEARCH_NAME, absoluteUrl } from "./seo";

// Its Image is the favicon, whose 16-pixel image is the size the spec suggests (a browser picks the one it wants
// from the ICO). Absolute, as a browser fetches it from a description it may have stored apart from the site.
export function opensearchXml(): string {
  return `<?xml version="1.0" encoding="UTF-8"?>
<OpenSearchDescription xmlns="http://a9.com/-/spec/opensearch/1.1/" xmlns:moz="http://www.mozilla.org/2006/browser/search/">
  <ShortName>${SEARCH_NAME}</ShortName>
  <Description>Search the unofficial Spelunky Classic wiki on spelunky.ai.</Description>
  <InputEncoding>UTF-8</InputEncoding>
  <Image width="16" height="16" type="image/x-icon">${absoluteUrl(FAVICON_PATH)}</Image>
  <Url type="text/html" method="get" template="${absoluteUrl("/wiki")}?q={searchTerms}"/>
  <Url type="${OPENSEARCH_TYPE}" rel="self" template="${absoluteUrl(OPENSEARCH_PATH)}"/>
  <moz:SearchForm>${absoluteUrl("/wiki")}</moz:SearchForm>
</OpenSearchDescription>
`;
}
