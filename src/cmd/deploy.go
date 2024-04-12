/*
Copyright © 2024 NAME HERE <EMAIL ADDRESS>
*/
package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

// deployCmd represents the deploy command
var deployCmd = &cobra.Command{
	Use:   "deploy",
	Short: "Deploy challenges to cluster",
	Long: `Deploy all challenges enabled for deloyment in rctf.yaml.

Builds and pushes images by default, unless --no-build is given.
	`,
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Println("deploy called")
	},
}

var noBuild bool
var dryRun bool

func init() {
	rootCmd.AddCommand(deployCmd)
	deployCmd.GroupID = "ours"

	deployCmd.Flags().BoolVar(&noBuild, "no-build", false, "skip building new images")
	deployCmd.Flags().BoolVar(&dryRun, "dry-run", false, "test changes without applying")
}
