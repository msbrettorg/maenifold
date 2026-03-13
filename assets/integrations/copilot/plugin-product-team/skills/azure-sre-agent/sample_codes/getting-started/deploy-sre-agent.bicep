// Deploy Azure SRE Agent via Bicep using Microsoft.App/agents resource type.
// Based on community patterns from:
//   - github.com/matthansen0/azure-sre-agent-sandbox
//   - github.com/dm-chelupati/sre-agent-lab
//
// API version: 2025-05-01-preview (preview resource type, use #disable-next-line BCP081)
// Regions: eastus2, swedencentral, australiaeast ONLY
//
// Usage:
//   az deployment group create \
//     --resource-group rg-sre-agent \
//     --template-file deploy-sre-agent.bicep \
//     --parameters agentName='my-sre-agent' \
//                  appInsightsAppId='<app-id>' \
//                  appInsightsConnectionString='<conn-string>'

@description('Name of the SRE Agent')
param agentName string

@description('Azure region — SRE Agent is ONLY available in these regions')
@allowed([
  'eastus2'
  'swedencentral'
  'australiaeast'
])
param location string = 'eastus2'

@description('Access level: High = Reader + Contributor + Log Analytics Reader, Low = Reader + Log Analytics Reader')
@allowed(['High', 'Low'])
param accessLevel string = 'High'

@description('Application Insights App ID')
param appInsightsAppId string

@description('Application Insights connection string')
param appInsightsConnectionString string

@description('Resource group ID to add as managed resource (for knowledge graph)')
param managedResourceGroupId string = resourceGroup().id

// User-Assigned Managed Identity
resource managedIdentity 'Microsoft.ManagedIdentity/userAssignedIdentities@2023-01-31' = {
  name: '${agentName}-identity'
  location: location
}

// Role definitions by access level
var roleDefinitions = {
  Low: [
    '92aaf0da-9dab-42b6-94a3-d43ce8d16293' // Log Analytics Reader
    'acdd72a7-3385-48ef-bd42-f606fba81ae7' // Reader
  ]
  High: [
    '92aaf0da-9dab-42b6-94a3-d43ce8d16293' // Log Analytics Reader
    'acdd72a7-3385-48ef-bd42-f606fba81ae7' // Reader
    'b24988ac-6180-42a0-ab88-20f7382dd24c' // Contributor
  ]
}

// RBAC role assignments for the managed identity
resource roleAssignments 'Microsoft.Authorization/roleAssignments@2022-04-01' = [
  for (roleId, index) in roleDefinitions[accessLevel]: {
    name: guid(resourceGroup().id, managedIdentity.id, roleId)
    properties: {
      roleDefinitionId: resourceId('Microsoft.Authorization/roleDefinitions', roleId)
      principalId: managedIdentity.properties.principalId
      principalType: 'ServicePrincipal'
    }
  }
]

// The SRE Agent itself
// Preview API — requires #disable-next-line BCP081 to suppress unknown-resource-type warning
#disable-next-line BCP081
resource sreAgent 'Microsoft.App/agents@2025-05-01-preview' = {
  name: agentName
  location: location
  identity: {
    type: 'SystemAssigned, UserAssigned'
    userAssignedIdentities: {
      '${managedIdentity.id}': {}
    }
  }
  properties: {
    knowledgeGraphConfiguration: {
      managedResources: [
        managedResourceGroupId
      ]
      identity: managedIdentity.id
    }
    actionConfiguration: {
      mode: 'Review'     // 'Review' = human approval, 'autonomous' = auto-act
      identity: managedIdentity.id
      accessLevel: accessLevel
    }
    mcpServers: []       // Configure MCP servers post-deploy via portal or REST API
    logConfiguration: {
      applicationInsightsConfiguration: {
        appId: appInsightsAppId
        connectionString: appInsightsConnectionString
      }
    }
  }
  dependsOn: [
    roleAssignments
  ]
}

// Grant deploying user SRE Agent Administrator role
resource sreAgentAdminRoleAssignment 'Microsoft.Authorization/roleAssignments@2022-04-01' = {
  name: guid(sreAgent.id, deployer().objectId, 'e79298df-d852-4c6d-84f9-5d13249d1e55')
  scope: sreAgent
  properties: {
    roleDefinitionId: resourceId(
      'Microsoft.Authorization/roleDefinitions',
      'e79298df-d852-4c6d-84f9-5d13249d1e55' // SRE Agent Administrator
    )
    principalId: deployer().objectId
    principalType: 'User'
  }
}

output agentName string = sreAgent.name
output agentId string = sreAgent.id
output agentEndpoint string = sreAgent.properties.agentEndpoint
output agentPortalUrl string = 'https://sre.azure.com'
output managedIdentityId string = managedIdentity.id
output managedIdentityPrincipalId string = managedIdentity.properties.principalId
