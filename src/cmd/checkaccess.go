/*
Copyright © 2024 NAME HERE <EMAIL ADDRESS>
*/
package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

// checkCmd represents the checkaccess command
var checkCmd = &cobra.Command{
	Use:     "check-access",
	Aliases: []string{"access"},
	Short:   "Make sure configured credentials are valid",
	Long: `Verifies that credentials set in the current profile are valid and work.

If no flags are given, check all credentials.
`,
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Println("checkaccess called")
	},
}

func init() {
	rootCmd.AddCommand(checkCmd)
	checkCmd.GroupID = "ours"

	checkCmd.Flags().BoolP("kubernetes", "k", true, "check kubernetes cluster access")
	checkCmd.Flags().BoolP("registry", "r", true, "check container registry access")
	checkCmd.Flags().BoolP("frontend", "f", true, "check rCTF frontend access")
}
