export interface IndexRange {
  start: number;
  end: number;
}

export interface PropertyValueRange extends IndexRange {
  key: string;
}
