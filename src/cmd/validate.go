/*
Copyright © 2024 NAME HERE <EMAIL ADDRESS>
*/
package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

// validateCmd represents the validate command
var validateCmd = &cobra.Command{
	Use:   "validate",
	Short: "Check for any errors in config files",
	Long: `Checks for errors in rcds.yaml and any challenge.yaml configurations.

	`,
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Println("validate called")
	},
}

func init() {
	rootCmd.AddCommand(validateCmd)
	validateCmd.GroupID = "ours"
}
