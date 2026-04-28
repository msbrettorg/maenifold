declare module 'markdown-it-footnote' {
  import type MarkdownIt from 'markdown-it';

  const markdownItFootnote: MarkdownIt.PluginSimple;
  export default markdownItFootnote;
}

declare module 'markdown-it-task-lists' {
  import type MarkdownIt from 'markdown-it';

  interface MarkdownItTaskListsOptions {
    enabled?: boolean;
    label?: boolean;
    labelAfter?: boolean;
  }

  const markdownItTaskLists: MarkdownIt.PluginWithOptions<MarkdownItTaskListsOptions>;
  export default markdownItTaskLists;
}
