/*
 * SPDX-License-Identifier: BSD-3-Clause
 * Copyright(c) 2017 Cavium, Inc
 */

#include "timvf_evdev.h"

int otx_logtype_timvf;

RTE_INIT(otx_timvf_init_log);
static void
otx_timvf_init_log(void)
{
	otx_logtype_timvf = rte_log_register("pmd.event.octeontx.timer");
	if (otx_logtype_timvf >= 0)
		rte_log_set_level(otx_logtype_timvf, RTE_LOG_NOTICE);
}

struct __rte_packed timvf_mbox_dev_info {
	uint64_t ring_active[4];
	uint64_t clk_freq;
};

/* Response messages */
enum {
	MBOX_RET_SUCCESS,
	MBOX_RET_INVALID,
	MBOX_RET_INTERNAL_ERR,
};

static int
timvf_mbox_dev_info_get(struct timvf_mbox_dev_info *info)
{
	struct octeontx_mbox_hdr hdr = {0};
	uint16_t len = sizeof(struct timvf_mbox_dev_info);

	hdr.coproc = TIM_COPROC;
	hdr.msg = TIM_GET_DEV_INFO;
	hdr.vfid = 0; /* TIM DEV is always 0. TIM RING ID changes. */

	memset(info, 0, len);
	return octeontx_mbox_send(&hdr, NULL, 0, info, len);
}

static void
timvf_ring_info_get(const struct rte_event_timer_adapter *adptr,
		struct rte_event_timer_adapter_info *adptr_info)
{
	struct timvf_ring *timr = adptr->data->adapter_priv;
	adptr_info->max_tmo_ns = timr->max_tout;
	adptr_info->min_resolution_ns = timr->tck_nsec;
	rte_memcpy(&adptr_info->conf, &adptr->data->conf,
			sizeof(struct rte_event_timer_adapter_conf));
}

static int
timvf_ring_conf_set(struct timvf_ctrl_reg *rctl, uint8_t ring_id)
{
	struct octeontx_mbox_hdr hdr = {0};
	uint16_t len = sizeof(struct timvf_ctrl_reg);
	int ret;

	hdr.coproc = TIM_COPROC;
	hdr.msg = TIM_SET_RING_INFO;
	hdr.vfid = ring_id;

	ret = octeontx_mbox_send(&hdr, rctl, len, NULL, 0);
	if (ret < 0 || hdr.res_code != MBOX_RET_SUCCESS)
		return -EACCES;
	return 0;
}

static int
timvf_get_start_cyc(uint64_t *now, uint8_t ring_id)
{
	struct octeontx_mbox_hdr hdr = {0};

	hdr.coproc = TIM_COPROC;
	hdr.msg = TIM_RING_START_CYC_GET;
	hdr.vfid = ring_id;
	*now = 0;
	return octeontx_mbox_send(&hdr, NULL, 0, now, sizeof(uint64_t));
}

static int
timvf_ring_start(const struct rte_event_timer_adapter *adptr)
{
	int ret;
	uint64_t interval;
	struct timvf_ctrl_reg rctrl;
	struct timvf_mbox_dev_info dinfo;
	struct timvf_ring *timr = adptr->data->adapter_priv;

	ret = timvf_mbox_dev_info_get(&dinfo);
	if (ret < 0 || ret != sizeof(struct timvf_mbox_dev_info))
		return -EINVAL;

	/* Calculate the interval cycles according to clock source. */
	switch (timr->clk_src) {
	case TIM_CLK_SRC_SCLK:
		interval = NSEC2CLK(timr->tck_nsec, dinfo.clk_freq);
		break;
	case TIM_CLK_SRC_GPIO:
		/* GPIO doesn't work on tck_nsec. */
		interval = 0;
		break;
	case TIM_CLK_SRC_GTI:
		interval = NSEC2CLK(timr->tck_nsec, dinfo.clk_freq);
		break;
	case TIM_CLK_SRC_PTP:
		interval = NSEC2CLK(timr->tck_nsec, dinfo.clk_freq);
		break;
	default:
		timvf_log_err("Unsupported clock source configured %d",
				timr->clk_src);
		return -EINVAL;
	}

	/*CTRL0 register.*/
	rctrl.rctrl0 = interval;

	/*CTRL1	register.*/
	rctrl.rctrl1 =	(uint64_t)(timr->clk_src) << 51 |
		1ull << 48 /* LOCK_EN (Enable hw bucket lock mechanism) */ |
		1ull << 47 /* ENA */ |
		1ull << 44 /* ENA_LDWB */ |
		(timr->nb_bkts - 1);

	rctrl.rctrl2 = (uint64_t)(TIM_CHUNK_SIZE / 16) << 40;

	timvf_write64((uintptr_t)timr->bkt,
			(uint8_t *)timr->vbar0 + TIM_VRING_BASE);
	if (timvf_ring_conf_set(&rctrl, timr->tim_ring_id)) {
		ret = -EACCES;
		goto error;
	}

	if (timvf_get_start_cyc(&timr->ring_start_cyc,
				timr->tim_ring_id) < 0) {
		ret = -EACCES;
		goto error;
	}
	timr->tck_int = NSEC2CLK(timr->tck_nsec, rte_get_timer_hz());
	timr->fast_div = rte_reciprocal_value_u64(timr->tck_int);
	timvf_log_info("nb_bkts %d min_ns %"PRIu64" min_cyc %"PRIu64""
			" maxtmo %"PRIu64"\n",
			timr->nb_bkts, timr->tck_nsec, interval,
			timr->max_tout);

	return 0;
error:
	rte_free(timr->bkt);
	rte_mempool_free(timr->chunk_pool);
	return ret;
}

static int
timvf_ring_stop(const struct rte_event_timer_adapter *adptr)
{
	struct timvf_ring *timr = adptr->data->adapter_priv;
	struct timvf_ctrl_reg rctrl = {0};
	rctrl.rctrl0 = timvf_read64((uint8_t *)timr->vbar0 + TIM_VRING_CTL0);
	rctrl.rctrl1 = timvf_read64((uint8_t *)timr->vbar0 + TIM_VRING_CTL1);
	rctrl.rctrl1 &= ~(1ull << 47); /* Disable */
	rctrl.rctrl2 = timvf_read64((uint8_t *)timr->vbar0 + TIM_VRING_CTL2);

	if (timvf_ring_conf_set(&rctrl, timr->tim_ring_id))
		return -EACCES;
	return 0;
}

static int
timvf_ring_create(struct rte_event_timer_adapter *adptr)
{
	char pool_name[25];
	int ret;
	uint64_t nb_timers;
	struct rte_event_timer_adapter_conf *rcfg = &adptr->data->conf;
	struct timvf_ring *timr;
	struct timvf_info tinfo;
	const char *mempool_ops;

	if (timvf_info(&tinfo) < 0)
		return -ENODEV;

	if (adptr->data->id >= tinfo.total_timvfs)
		return -ENODEV;

	timr = rte_zmalloc("octeontx_timvf_priv",
			sizeof(struct timvf_ring), 0);
	if (timr == NULL)
		return -ENOMEM;

	adptr->data->adapter_priv = timr;
	/* Check config parameters. */
	if ((rcfg->clk_src != RTE_EVENT_TIMER_ADAPTER_CPU_CLK) &&
			(!rcfg->timer_tick_ns ||
			 rcfg->timer_tick_ns < TIM_MIN_INTERVAL)) {
		timvf_log_err("Too low timer ticks");
		goto cfg_err;
	}

	timr->clk_src = (int) rcfg->clk_src;
	timr->tim_ring_id = adptr->data->id;
	timr->tck_nsec = rcfg->timer_tick_ns;
	timr->max_tout = rcfg->max_tmo_ns;
	timr->nb_bkts = (timr->max_tout / timr->tck_nsec);
	timr->vbar0 = timvf_bar(timr->tim_ring_id, 0);
	timr->bkt_pos = (uint8_t *)timr->vbar0 + TIM_VRING_REL;
	nb_timers = rcfg->nb_timers;
	timr->get_target_bkt = bkt_mod;

	timr->nb_chunks = nb_timers / nb_chunk_slots;

	timr->bkt = rte_zmalloc("octeontx_timvf_bucket",
			(timr->nb_bkts) * sizeof(struct tim_mem_bucket),
			0);
	if (timr->bkt == NULL)
		goto mem_err;

	snprintf(pool_name, 30, "timvf_chunk_pool%d", timr->tim_ring_id);
	timr->chunk_pool = (void *)rte_mempool_create_empty(pool_name,
			timr->nb_chunks, TIM_CHUNK_SIZE, 0, 0, rte_socket_id(),
			0);

	if (!timr->chunk_pool) {
		rte_free(timr->bkt);
		timvf_log_err("Unable to create chunkpool.");
		return -ENOMEM;
	}

	mempool_ops = rte_mbuf_best_mempool_ops();
	ret = rte_mempool_set_ops_byname(timr->chunk_pool,
			mempool_ops, NULL);

	if (ret != 0) {
		timvf_log_err("Unable to set chunkpool ops.");
		goto mem_err;
	}

	ret = rte_mempool_populate_default(timr->chunk_pool);
	if (ret < 0) {
		timvf_log_err("Unable to set populate chunkpool.");
		goto mem_err;
	}
	timvf_write64(0, (uint8_t *)timr->vbar0 + TIM_VRING_BASE);
	timvf_write64(0, (uint8_t *)timr->vbar0 + TIM_VF_NRSPERR_INT);
	timvf_write64(0, (uint8_t *)timr->vbar0 + TIM_VF_NRSPERR_INT_W1S);
	timvf_write64(0x7, (uint8_t *)timr->vbar0 + TIM_VF_NRSPERR_ENA_W1C);
	timvf_write64(0x7, (uint8_t *)timr->vbar0 + TIM_VF_NRSPERR_ENA_W1S);

	return 0;
mem_err:
	rte_free(timr);
	return -ENOMEM;
cfg_err:
	rte_free(timr);
	return -EINVAL;
}

static int
timvf_ring_free(struct rte_event_timer_adapter *adptr)
{
	struct timvf_ring *timr = adptr->data->adapter_priv;
	rte_mempool_free(timr->chunk_pool);
	rte_free(timr->bkt);
	rte_free(adptr->data->adapter_priv);
	return 0;
}

static int
timvf_stats_get(const struct rte_event_timer_adapter *adapter,
		struct rte_event_timer_adapter_stats *stats)
{
	struct timvf_ring *timr = adapter->data->adapter_priv;
	uint64_t bkt_cyc = rte_rdtsc() - timr->ring_start_cyc;

	stats->evtim_exp_count = timr->tim_arm_cnt;
	stats->ev_enq_count = timr->tim_arm_cnt;
	stats->adapter_tick_count = rte_reciprocal_divide_u64(bkt_cyc,
				&timr->fast_div);
	return 0;
}

static int
timvf_stats_reset(const struct rte_event_timer_adapter *adapter)
{
	struct timvf_ring *timr = adapter->data->adapter_priv;

	timr->tim_arm_cnt = 0;
	return 0;
}

static struct rte_event_timer_adapter_ops timvf_ops = {
	.init		= timvf_ring_create,
	.uninit		= timvf_ring_free,
	.start		= timvf_ring_start,
	.stop		= timvf_ring_stop,
	.get_info	= timvf_ring_info_get,
};

int
timvf_timer_adapter_caps_get(const struct rte_eventdev *dev, uint64_t flags,
		uint32_t *caps, const struct rte_event_timer_adapter_ops **ops,
		uint8_t enable_stats)
{
	RTE_SET_USED(dev);
	RTE_SET_USED(flags);

	if (enable_stats) {
		timvf_ops.stats_get   = timvf_stats_get;
		timvf_ops.stats_reset = timvf_stats_reset;
	}

	*caps = RTE_EVENT_TIMER_ADAPTER_CAP_INTERNAL_PORT;
	*ops = &timvf_ops;
	return -EINVAL;
}
