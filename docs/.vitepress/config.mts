import { defineConfig } from "vitepress";
import { generateSidebar } from "vitepress-sidebar";

// https://vitepress.dev/reference/site-config
export default defineConfig({
  title: "beaverCDS Docs",
  description: "Next-generation CTF deployment framework",
  themeConfig: {
    // https://vitepress.dev/reference/default-theme-config
    nav: [
      { text: "Home", link: "/" },
      // { text: "Examples", link: "/markdown-examples" },
    ],

    // auto generate sidebar from directory structure, via vitepress-sidebar
    sidebar: generateSidebar({
      documentRootPath: "./",
      // pull title from markdown not filename
      useTitleFromFileHeading: true,
      useTitleFromFrontmatter: true,
      keepMarkdownSyntaxFromTitle: true,
      // transform name to sentence case
      hyphenToSpace: true,
      underscoreToSpace: true,
      capitalizeEachWords: true,
    }),

    socialLinks: [
      { icon: "github", link: "https://github.com/osusec/beavercds-ng" },
    ],
  },

  // disable interpolation of {{ and }} in markdown
  markdown: {
    config(md) {
      const defaultCodeInline = md.renderer.rules.code_inline!;
      md.renderer.rules.code_inline = (tokens, idx, options, env, self) => {
        tokens[idx].attrSet("v-pre", "");
        return defaultCodeInline(tokens, idx, options, env, self);
      };
    },
  },
});
