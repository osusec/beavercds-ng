/*
Copyright © 2024 NAME HERE <EMAIL ADDRESS>
*/
package lib

import (
	"errors"
	"fmt"
	"os"

	"gopkg.in/yaml.v3"
)

type UserPass struct {
	User string
	Pass string
}

type ChallengePoints struct {
	Difficulty int
	Min        int
	Max        int
}

type ProfileConfig struct {
	Name               string
	DeployedChallenges map[string]bool
	FrontendUrl        string
	FrontendToken      string
	ChallengesDomain   string
	KubeConfig         string
	KubeContext        string
}

type RCDSConfig struct {
	FlagRegex string
	Registry  struct {
		Domain  string
		Build   UserPass
		Cluster UserPass
	}
	Defaults struct {
		Difficulty int
		Resources  struct {
			Cpu    float64
			Memory string
		}
	}
	Profiles map[string]ProfileConfig

	Points []ChallengePoints
}

func ParseRCDS(path string) (*RCDSConfig, error) {

	var config RCDSConfig

	data, err := os.ReadFile(path)
	if err != nil {
		fmt.Println(err)
		return nil, errors.New("Cannot open file")
	}

	var parsed map[string]interface{}

	err = yaml.Unmarshal(data, &parsed)
	if err != nil {
		fmt.Println("Error unmarshalling")
		return nil, errors.New("Error unmarshalling")
	}

	if value, ok := parsed["flag_regex"].(string); ok {
		config.FlagRegex = value
	}

	if registry_map, ok := parsed["registry"].(map[string]interface{}); ok {

		if domain, ok := registry_map["domain"].(string); ok {
			config.Registry.Domain = domain
		}

		// There can be a top-level user/pass pair, or for build/cluster
		user, user_ok := registry_map["user"].(string)
		pass, pass_ok := registry_map["pass"].(string)

		if user_ok && pass_ok {
			config.Registry.Build.User = user
			config.Registry.Build.Pass = pass
			config.Registry.Cluster.User = user
			config.Registry.Cluster.Pass = pass
		} else {
			if build_map, ok := registry_map["build"].(map[string]string); ok {

				user, user_ok := build_map["user"]
				pass, pass_ok := build_map["pass"]

				if user_ok && pass_ok {
					config.Registry.Build.User = user
					config.Registry.Build.Pass = pass
				}
			}

			if cluster_map, ok := registry_map["build"].(map[string]string); ok {

				user, user_ok := cluster_map["user"]
				pass, pass_ok := cluster_map["pass"]

				if user_ok && pass_ok {
					config.Registry.Cluster.User = user
					config.Registry.Cluster.Pass = pass
				}
			}
		}
	}

	if default_map, ok := parsed["defaults"].(map[string]interface{}); ok {
		if difficulty, ok := default_map["difficulty"].(int); ok {
			config.Defaults.Difficulty = difficulty
		}

		if resource_map, ok := default_map["resources"].(map[string]interface{}); ok {
			if cpu, ok := resource_map["cpu"].(float64); ok {
				config.Defaults.Resources.Cpu = cpu
			}

			if memory, ok := resource_map["cpu"].(string); ok {
				config.Defaults.Resources.Memory = memory
			}
		}
	}

	points_list, ok := parsed["points"].([]interface{})
	if ok {
		for _, element_map_1 := range points_list {
			element_map, _ := element_map_1.(map[string]interface{})

			// index is the index where we are
			// element is the element from someSlice for where we are

			var chalpoints ChallengePoints

			if num, ok := element_map["difficulty"].(int); ok {
				chalpoints.Difficulty = num
			}

			if min, ok := element_map["min"].(int); ok {
				chalpoints.Min = min
			}

			if max, ok := element_map["max"].(int); ok {
				chalpoints.Max = max
			}

			config.Points = append(config.Points, chalpoints)
		}
	}

	// Initialize profiles
	config.Profiles = make(map[string]ProfileConfig)

	if deploy_map, ok := parsed["profiles"].(map[string]interface{}); ok {
		for profile_name, profile_map := range deploy_map {

			var profile ProfileConfig
			profile.Name = profile_name
			profile.DeployedChallenges = make(map[string]bool)

			if profile_map, ok := profile_map.(map[string]interface{}); ok {

				if frontend_url, ok := profile_map["frontend_url"].(string); ok {
					profile.FrontendUrl = frontend_url
				}

				if frontend_token, ok := profile_map["frontend_token"].(string); ok {
					profile.FrontendToken = frontend_token
				}

				if challenges_domain, ok := profile_map["challenges_domain"].(string); ok {
					profile.ChallengesDomain = challenges_domain
				}

				if kubeconfig, ok := profile_map["kubeconfig"].(string); ok {
					profile.KubeConfig = kubeconfig
				}

				if kubecontext, ok := profile_map["kubecontext"].(string); ok {
					profile.KubeContext = kubecontext
				}
			}

			config.Profiles[profile_name] = profile
		}
	}

	if deploy_map, ok := parsed["deploy"].(map[string]interface{}); ok {
		for profile_name, profile_map := range deploy_map {

			if profile, ok := config.Profiles[profile_name]; ok {

				if profile_map, ok := profile_map.(map[string]interface{}); ok {
					for challenge_name, deployed := range profile_map {
						if deployed, ok := deployed.(bool); ok {
							profile.DeployedChallenges[challenge_name] = deployed
						}
					}
				}

				config.Profiles[profile.Name] = profile
			}

		}
	}

	// fmt.Printf("%+v\n", config)
	return &config, nil
}
