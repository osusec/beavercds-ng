/*
Copyright © 2024 NAME HERE <EMAIL ADDRESS>
*/
package cmd

import (
	"os"

	cc "github.com/ivanpirog/coloredcobra"
	"github.com/spf13/cobra"
)

// rootCmd represents the base command when called without any subcommands
var rootCmd = &cobra.Command{
	Use:   "beavercds-ng",
	Short: "kubernetes ctf challenge deployer",
	Long: `Deployment manager for rCTF/beaverCTF challenges deployed on Kubernetes.

	`,
	// Uncomment the following line if your bare application
	// has an action associated with it:
	// Run: func(cmd *cobra.Command, args []string) { },
}

var verbose bool
var profileName string

// Execute adds all child commands to the root command and sets flags appropriately.
// This is called by main.main(). It only needs to happen once to the rootCmd.
func Execute() {
	// setup colored cobra
	cc.Init(&cc.Config{
		RootCmd:         rootCmd,
		Commands:        cc.Bold,
		Example:         cc.Italic,
		ExecName:        cc.Bold,
		Flags:           cc.Bold,
		NoExtraNewlines: true,
	})

	err := rootCmd.Execute()
	if err != nil {
		os.Exit(1)
	}
}

func init() {
	// persistent flags are global for your application.

	// verbose / log-level
	rootCmd.PersistentFlags().BoolVarP(&verbose, "verbose", "v", false, "show verbose output")

	// profile selection
	rootCmd.PersistentFlags().StringVarP(&profileName, "profile", "p", "", "deployment profile to use")
	rootCmd.MarkPersistentFlagRequired("profile")

	// group our commands together
	rootCmd.AddGroup(&cobra.Group{ID: "ours", Title: "Commands:"})

	// put meta commands in their own group after
	rootCmd.AddGroup(&cobra.Group{ID: "meta", Title: "Other Commands:"})
	rootCmd.SetHelpCommandGroupID("meta")
	rootCmd.SetCompletionCommandGroupID("meta")
}
