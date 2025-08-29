/**
 * Validates if a string is a valid domain name, IP address, or localhost
 */
export function validateDomain(domain: string): boolean {
  // Basic domain validation
  const domainRegex = /^([a-z0-9]+(-[a-z0-9]+)*\.)+[a-z]{2,}$/i
  const ipRegex = /^(\d{1,3}\.){3}\d{1,3}(:\d{1,5})?$/
  const localhostRegex = /^localhost(:\d{1,5})?$/i
  
  return domainRegex.test(domain) || ipRegex.test(domain) || localhostRegex.test(domain)
}

/**
 * Compares two domain arrays to check if they're different
 */
export function hasDomainsChanged(domains1: string[], domains2: string[]): boolean {
  return JSON.stringify(domains1.sort()) !== JSON.stringify(domains2.sort())
}