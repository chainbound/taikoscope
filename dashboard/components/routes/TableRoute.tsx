import React, { useEffect, useState } from 'react';
import { useParams, useSearchParams, useOutletContext, useNavigate } from 'react-router-dom';
import { TableView } from '../views/TableView';
import { useTableActions, TableViewState } from '../../hooks/useTableActions';
import { useRefreshTimer } from '../../hooks/useRefreshTimer';
import { TimeRange } from '../../types';

interface DashboardContextType {
  timeRange: TimeRange;
  setTimeRange: (range: TimeRange) => void;
  selectedSequencer: string | null;
  setSelectedSequencer: (seq: string | null) => void;
  sequencerList: string[];
  chartsData: any;
  blockData: any;
  metricsData: any;
}

export const TableRoute: React.FC = () => {
  const { tableType } = useParams<{ tableType: string }>();
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  
  const {
    timeRange,
    setTimeRange,
    selectedSequencer,
    setSelectedSequencer,
    sequencerList,
    chartsData,
    metricsData,
  } = useOutletContext<DashboardContextType>();

  const [tableView] = useState<TableViewState | undefined>(undefined);
  const [tableLoading, setTableLoading] = useState(false);
  const refreshTimer = useRefreshTimer();

  const {
    openGenericTable,
    openTpsTable,
    openSequencerDistributionTable,
  } = useTableActions(
    timeRange,
    setTimeRange,
    selectedSequencer,
    chartsData.blockTxData,
    chartsData.l2BlockTimeData,
  );

  useEffect(() => {
    const loadTable = async () => {
      if (!tableType) return;
      
      setTableLoading(true);
      
      try {
        const range = (searchParams.get('range') as TimeRange) || timeRange;
        
        switch (tableType) {
          case 'sequencer-blocks': {
            const address = searchParams.get('address');
            if (address) {
              await openGenericTable('sequencer-blocks', range, { address });
            }
            break;
          }
          case 'tps':
            openTpsTable();
            break;
          case 'sequencer-dist': {
            const pageStr = searchParams.get('page') ?? '0';
            const page = parseInt(pageStr, 10);
            if (!isNaN(page) && page >= 0) {
              const start = searchParams.get('start');
              const end = searchParams.get('end');
              await openSequencerDistributionTable(
                range,
                page,
                start ? Number(start) : undefined,
                end ? Number(end) : undefined,
              );
            }
            break;
          }
          default:
            await openGenericTable(tableType, range);
            break;
        }
      } catch (error) {
        console.error('Failed to load table:', error);
        metricsData.setErrorMessage(`Failed to load ${tableType} table. Please try again.`);
      } finally {
        setTableLoading(false);
      }
    };

    loadTable();
  }, [tableType, searchParams, timeRange, openGenericTable, openTpsTable, openSequencerDistributionTable, metricsData]);

  const handleBack = () => {
    navigate('/');
  };

  const handleManualRefresh = () => {
    // Trigger table reload by updating a dependency
    setTableLoading(true);
    setTimeout(() => setTableLoading(false), 100);
  };

  if (!tableView && !tableLoading) {
    return <div>Table not found</div>;
  }

  return (
    <TableView
      tableView={tableView}
      tableLoading={tableLoading}
      isNavigating={false}
      refreshTimer={refreshTimer}
      sequencerList={sequencerList}
      selectedSequencer={selectedSequencer}
      onSequencerChange={setSelectedSequencer}
      onBack={handleBack}
      onManualRefresh={handleManualRefresh}
    />
  );
};