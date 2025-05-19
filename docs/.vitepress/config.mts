import { defineConfig } from "vitepress";

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

    sidebar: [
      {
        text: "Guides",
        items: [
          { text: "Deployment Quickstart", link: "for-sysadmins/quickstart" },
          { text: "Add new challenge", link: "for-authors/quickstart" },
        ],
      },

      {
        text: "Infra Setup",
        items: [
          { text: "Quickstart", link: "/for-sysadmins/quickstart" },
          { text: "Install", link: "/for-sysadmins/quickstart" },
          { text: "Config Reference", link: "/for-sysadmins/config" },
          { text: "Architecture", link: "/for-sysadmins/architecture" },
        ],
      },
      {
        text: "For Authors",
        items: [
          { text: "Challenge Quickstart", link: "/for-authors/quickstart" },
          {
            text: "Challenge Config Reference",
            link: "/for-authors/challenge-config",
          },
        ],
      },
    ],

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
