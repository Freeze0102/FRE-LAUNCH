import type { QueryKey, QueryOptions } from '@tanstack/vue-query'

type FactoryOptions<TData> = Omit<QueryOptions<TData, Error, TData, TData, QueryKey>, 'queryKey' | 'select'> & { queryKey: QueryKey }

// Factories share a cache-key domain and leave selectors to each observer.
export function queryOptions<TData>(options: FactoryOptions<TData>): FactoryOptions<TData> {
	return options
}
