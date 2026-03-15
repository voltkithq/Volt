import type { EnterprisePolicyDefinition, EnterprisePolicySchema } from '../enterprise-schema.js';

export function generateAdmx(schema: EnterprisePolicySchema): string {
  const categoryId = 'CategoryVoltEnterprise';
  const policyEntries = schema.policies.map((policy) => renderAdmxPolicy(policy)).join('\n');

  return `<?xml version="1.0" encoding="utf-8"?>
<policyDefinitions revision="1.0" schemaVersion="1.0">
  <policyNamespaces>
    <target namespace="${escapeXml(schema.namespace)}" prefix="volt" />
    <using namespace="Microsoft.Policies.Windows" prefix="windows" />
  </policyNamespaces>
  <resources minRequiredRevision="1.0" />
  <categories>
    <category name="${categoryId}" displayName="$(string.${categoryId})" />
  </categories>
  <policies>
${policyEntries}
  </policies>
</policyDefinitions>
`;
}

export function generateAdml(schema: EnterprisePolicySchema): string {
  const categoryId = 'CategoryVoltEnterprise';
  const stringRows: string[] = [
    `      <string id="${categoryId}">${escapeXml(schema.categoryDisplayName)}</string>`,
  ];
  const presentationRows: string[] = [];

  for (const policy of schema.policies) {
    stringRows.push(
      `      <string id="Policy_${policy.id}">${escapeXml(policy.displayName)}</string>`,
    );
    stringRows.push(
      `      <string id="Policy_${policy.id}_Help">${escapeXml(policy.description)}</string>`,
    );

    if (policy.type === 'enum') {
      for (const option of policy.enumValues ?? []) {
        stringRows.push(
          `      <string id="Policy_${policy.id}_${option.id}">${escapeXml(option.displayName)}</string>`,
        );
      }
    }

    const presentation = renderAdmlPresentation(policy);
    if (presentation) {
      presentationRows.push(presentation);
    }
  }

  return `<?xml version="1.0" encoding="utf-8"?>
<policyDefinitionResources revision="1.0" schemaVersion="1.0">
  <displayName>Volt Enterprise Policies</displayName>
  <description>Group Policy templates for Volt-managed desktop deployments.</description>
  <resources>
    <stringTable>
${stringRows.join('\n')}
    </stringTable>
    <presentationTable>
${presentationRows.join('\n')}
    </presentationTable>
  </resources>
</policyDefinitionResources>
`;
}

function renderAdmxPolicy(policy: EnterprisePolicyDefinition): string {
  const escapedId = escapeXml(policy.id);
  const escapedKey = escapeXml(policy.registryKey);
  const escapedValueName = escapeXml(policy.valueName);
  const presentation =
    policy.type === 'boolean' ? '' : ` presentation="$(presentation.Pres_${escapedId})"`;

  return `    <policy name="${escapedId}" class="Machine" displayName="$(string.Policy_${escapedId})" explainText="$(string.Policy_${escapedId}_Help)" key="${escapedKey}" valueName="${escapedValueName}"${presentation}>
${renderAdmxPolicyBody(policy)}
      <parentCategory ref="CategoryVoltEnterprise" />
    </policy>`;
}

function renderAdmxPolicyBody(policy: EnterprisePolicyDefinition): string {
  if (policy.type === 'boolean') {
    return [
      '      <enabledValue><decimal value="1" /></enabledValue>',
      '      <disabledValue><decimal value="0" /></disabledValue>',
    ].join('\n');
  }

  if (policy.type === 'text') {
    return [
      '      <elements>',
      `        <text id="${escapeXml(policy.id)}" valueName="${escapeXml(policy.valueName)}" required="true" />`,
      '      </elements>',
    ].join('\n');
  }

  if (policy.type === 'decimal') {
    const minValue = Number.isFinite(policy.minValue) ? policy.minValue : 1;
    const maxValue = Number.isFinite(policy.maxValue) ? policy.maxValue : 9999;
    return [
      '      <elements>',
      `        <decimal id="${escapeXml(policy.id)}" valueName="${escapeXml(policy.valueName)}" minValue="${minValue}" maxValue="${maxValue}" />`,
      '      </elements>',
    ].join('\n');
  }

  const enumItems = (policy.enumValues ?? [])
    .map((option) =>
      [
        `          <item displayName="$(string.Policy_${escapeXml(policy.id)}_${escapeXml(option.id)})">`,
        `            <value><string>${escapeXml(option.value)}</string></value>`,
        '          </item>',
      ].join('\n'),
    )
    .join('\n');

  return [
    '      <elements>',
    `        <enum id="${escapeXml(policy.id)}" valueName="${escapeXml(policy.valueName)}">`,
    enumItems,
    '        </enum>',
    '      </elements>',
  ].join('\n');
}

function renderAdmlPresentation(policy: EnterprisePolicyDefinition): string | null {
  const presentationId = `Pres_${policy.id}`;
  if (policy.type === 'text') {
    const defaultValue =
      typeof policy.defaultValue === 'string' ? escapeXml(policy.defaultValue) : '';
    return [
      `      <presentation id="${escapeXml(presentationId)}">`,
      `        <textBox refId="${escapeXml(policy.id)}" defaultValue="${defaultValue}" />`,
      '      </presentation>',
    ].join('\n');
  }

  if (policy.type === 'decimal') {
    const defaultValue = typeof policy.defaultValue === 'number' ? policy.defaultValue : 0;
    return [
      `      <presentation id="${escapeXml(presentationId)}">`,
      `        <decimalTextBox refId="${escapeXml(policy.id)}" defaultValue="${defaultValue}" spin="1" />`,
      '      </presentation>',
    ].join('\n');
  }

  if (policy.type === 'enum') {
    return [
      `      <presentation id="${escapeXml(presentationId)}">`,
      `        <dropdownList refId="${escapeXml(policy.id)}" />`,
      '      </presentation>',
    ].join('\n');
  }

  return null;
}

function escapeXml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&apos;');
}
