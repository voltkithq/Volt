# Data

Native-backed data helpers for heavy query and profiling workloads.

No additional permission is required.

Use these APIs from the renderer through `voltkit/renderer`:

```ts
import { data } from 'voltkit/renderer';
```

Volt routes these calls through reserved `volt:native:*` IPC channels and executes the heavy work in Rust instead of Boa handler code.

## `data.profile(options?): Promise<DataProfile>`

Build a profile for a synthetic in-memory dataset and return category/region spreads.

```ts
const profile = await data.profile({ datasetSize: 12_000 });
console.log(profile.cachedSizes);
```

### `DataProfileOptions`

```ts
interface DataProfileOptions {
  datasetSize?: number;
}
```

### `DataProfile`

```ts
interface DataProfile {
  datasetSize: number;
  cachedSizes: number[];
  categorySpread: Record<string, number>;
  regionSpread: Record<string, number>;
}
```

## `data.query(options?): Promise<DataQueryResult>`

Run a native data query/aggregation pass and return timing plus sample output.

```ts
const result = await data.query({
  datasetSize: 25_000,
  iterations: 6,
  searchTerm: 'risk',
  minScore: 61,
  topN: 24,
});

console.log(result.backendDurationMs);
console.log(result.categoryWinners);
```

### `DataQueryOptions`

```ts
interface DataQueryOptions {
  datasetSize?: number;
  iterations?: number;
  searchTerm?: string;
  minScore?: number;
  topN?: number;
}
```

### `DataQueryResult`

```ts
interface DataQueryResult {
  datasetSize: number;
  iterations: number;
  query: string;
  minScore: number;
  topN: number;
  backendDurationMs: number;
  filterDurationMs: number;
  sortDurationMs: number;
  aggregateDurationMs: number;
  peakMatches: number;
  totalMatchesAcrossIterations: number;
  categoryWinners: Array<{ category: string; total: number }>;
  sample: Array<{
    id: number;
    title: string;
    category: string;
    region: string;
    score: number;
    revenue: number;
    margin: number;
  }>;
  payloadBytes: number;
}
```

## Notes

- `data.*` is intended for coarse, heavy operations that should bypass Boa handler logic.
- The underlying `volt:native:*` channel names are reserved implementation details; app code should call `data.profile()` / `data.query()` instead of invoking those channels directly.
