import type { RequestResult } from '../services/apiService';

export const hasBadRequest = (results: RequestResult<unknown>[]): boolean =>
    results.some((r) => r.badRequest);

export const getErrorMessage = (anyBadRequest: boolean): string => {
    return anyBadRequest
        ? 'Invalid parameters provided. Some data may not be available.'
        : '';
};