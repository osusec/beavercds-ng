/*
Copyright © 2024 NAME HERE <EMAIL ADDRESS>
*/
package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

// buildCmd represents the build command
var buildCmd = &cobra.Command{
	Use:   "build",
	Short: "Build challenge images",
	Long: `Build container images for all challenges enabled for deloyment in rctf.yaml,
and optionally push images to the configured registry.

Images are tagged as <registry>/<chal>-<cntr>:<profile>.
	`,

	Args: cobra.ArbitraryArgs,
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Println("build called")
	},
}

var push bool

func init() {
	rootCmd.AddCommand(buildCmd)
	buildCmd.GroupID = "ours"

	// Here you will define your flags and configuration settings.
	buildCmd.Flags().BoolVar(&push, "push", false, "push newly built images")

}
