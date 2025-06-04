# API Design
The API is organized around REST principles. It accepts JSON-encoded request bodies, returns JSON-encoded responses, and uses standard HTTP response codes.

### Slot Range Parameters

Slot range parameters cannot be used together with time range parameters.

Note that slot range parameters are slightly different than time range parameters. The inclusion request for slot x might have been submitted well before slot x's actual time. A request with `slot[gte]=x` will return items targeting slot x and above, regardless of when those requests were created.

| Parameter   | Type    | Required | Description                                                 |
| ----------- | ------- | -------- | ----------------------------------------------------------- |
| `slot[gt]`  | integer | No       | Return results from slots **after** this slot number        |
| `slot[gte]` | integer | No       | Return results from slots **at or after** this slot number  |
| `slot[lt]`  | integer | No       | Return results from slots **before** this slot number       |
| `slot[lte]` | integer | No       | Return results from slots **at or before** this slot number |

### Time Range Parameters

All time range parameters are in milliseconds. Time range parameters cannot be used together with slot range parameters.

| Parameter      | Type    | Required | Description                                                 |
| -------------- | ------- | -------- | ----------------------------------------------------------- |
| `created[gt]`  | integer | No       | Return results created **after** this Unix timestamp        |
| `created[gte]` | integer | No       | Return results created **at or after** this Unix timestamp  |
| `created[lt]`  | integer | No       | Return results created **before** this Unix timestamp       |
| `created[lte]` | integer | No       | Return results created **at or before** this Unix timestamp |