import { createContext, useContext } from 'react';
import type { OS } from './site';

/** The OS the visitor is viewing (detected, or picked with the nav toggle). Mock windows read it for their chrome. */
export const OSContext = createContext<OS>('mac');
export const useOS = () => useContext(OSContext);
