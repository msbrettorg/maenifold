// Deploy Azure Bot Service for SRE Agent Teams integration
// This template creates the Bot Service resource and Teams channel connection.
//
// Required parameters:
//   botAppClientId  - Your agent's managed identity Client ID
//   botAppDomain    - Your agent's container app FQDN (e.g., my-agent.eastus2.azuresre.ai)
//   botDisplayName  - Name shown in Teams
//   tenantId        - Your Microsoft Entra tenant ID
//
// Usage:
//   az deployment group create \
//     --resource-group rg-sre-agent \
//     --template-file deploy-teams-bot.bicep \
//     --parameters botAppClientId='<client-id>' \
//                  botAppDomain='<agent-fqdn>' \
//                  botDisplayName='SRE Agent' \
//                  tenantId='<tenant-id>'

@description('Managed identity Client ID for the SRE Agent')
param botAppClientId string

@description('SRE Agent container app FQDN')
param botAppDomain string

@description('Display name for the Teams bot')
param botDisplayName string = 'SRE Agent'

@description('Microsoft Entra tenant ID')
param tenantId string

@description('Azure region for Bot Service')
param location string = resourceGroup().location

@allowed(['UserAssignedMsi', 'SingleTenant', 'MultiTenant'])
@description('Bot authentication type — UserAssignedMsi recommended for managed identity')
param appType string = 'UserAssignedMsi'

resource bot 'Microsoft.BotService/botServices@2022-09-15' = {
  name: botDisplayName
  location: 'global'
  kind: 'azurebot'
  sku: {
    name: 'S1'
  }
  properties: {
    displayName: botDisplayName
    endpoint: 'https://${botAppDomain}/api/messages'
    msaAppId: botAppClientId
    msaAppType: appType
    msaAppTenantId: tenantId
  }
}

resource teamsChannel 'Microsoft.BotService/botServices/channels@2022-09-15' = {
  parent: bot
  name: 'MsTeamsChannel'
  location: 'global'
  properties: {
    channelName: 'MsTeamsChannel'
    properties: {
      isEnabled: true
    }
  }
}

output botId string = bot.id
output botEndpoint string = bot.properties.endpoint
